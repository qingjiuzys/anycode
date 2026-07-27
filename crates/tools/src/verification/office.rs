//! Office artifact validators (DOCX / PPTX / XLSX openability + structure).

use super::{ArtifactValidator, ValidationContext};
use anycode_core::{
    Artifact, ExpectedArtifact, GateRequirement, VerificationOutcome, VerificationResult,
};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

fn first_path(candidates: &[Artifact], suffixes: &[&str]) -> Option<String> {
    for art in candidates {
        let Some(path) = art.path.as_deref() else {
            continue; // a candidate without a path must not hide later valid ones
        };
        let lower = path.to_ascii_lowercase();
        if suffixes.iter().any(|s| lower.ends_with(s)) {
            return Some(path.to_string());
        }
    }
    candidates.iter().find_map(|a| a.path.clone())
}

fn zip_openable(path: &Path) -> Result<(), String> {
    // Really try to open the archive — a 2-byte "PK" check lets forged files
    // pass the P0 gate.
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    zip::ZipArchive::new(file).map_err(|e| format!("not openable as ZIP/OOXML: {e}"))?;
    Ok(())
}

fn zip_contains(path: &Path, member: &str) -> Result<bool, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let ok = archive.by_name(member).is_ok();
    Ok(ok)
}

fn zip_read_string(path: &Path, member: &str) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut entry = archive.by_name(member).map_err(|e| e.to_string())?;
    let mut buf = String::new();
    entry.read_to_string(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

fn soffice_available() -> bool {
    which("soffice").is_some() || which("libreoffice").is_some()
}

fn find_repo_root(start: &Path) -> Option<PathBuf> {
    let marker = |p: &Path| {
        p.join("scripts/office/validate_slide_html.py").is_file()
            || p.join("scripts/office/render_pptx_evidence.py").is_file()
    };
    let mut cur = start.to_path_buf();
    loop {
        if marker(&cur) {
            return Some(cur);
        }
        if !cur.pop() {
            break;
        }
    }
    std::env::var("ANYCODE_REPO_ROOT")
        .ok()
        .map(PathBuf::from)
        .filter(|p| marker(p))
}

fn run_office_script(
    workspace: &Path,
    script_rel: &str,
    args: &[&str],
) -> Result<(), (String, String)> {
    let repo = find_repo_root(workspace).ok_or_else(|| {
        (
            "repo_root_missing".into(),
            format!("cannot find repo root from {}", workspace.display()),
        )
    })?;
    let script = repo.join(script_rel);
    if !script.is_file() {
        return Err((
            "script_missing".into(),
            format!("missing {}", script.display()),
        ));
    }
    let output = Command::new("python3")
        .arg(&script)
        .args(args)
        .current_dir(workspace)
        .output()
        .map_err(|e| ("spawn_failed".into(), e.to_string()))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Err((
            "validate_failed".into(),
            if stderr.is_empty() { stdout } else { stderr },
        ))
    }
}

fn find_slides_dir(workspace: &Path, candidates: &[Artifact]) -> Option<PathBuf> {
    for art in candidates {
        if let Some(path) = art.path.as_deref() {
            let p = Path::new(path);
            if p.is_file() && p.extension().and_then(|e| e.to_str()) == Some("html") {
                if let Some(parent) = p.parent() {
                    return Some(parent.to_path_buf());
                }
            }
            if p.is_dir() {
                return Some(p.to_path_buf());
            }
        }
    }
    for rel in ["slides", "."] {
        let dir = if rel == "." {
            workspace.to_path_buf()
        } else {
            workspace.join(rel)
        };
        if !dir.is_dir() {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let has_html = entries
            .flatten()
            .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("html"));
        if has_html {
            return Some(dir);
        }
    }
    None
}

fn find_report_md(workspace: &Path) -> Option<PathBuf> {
    for name in ["report.md", "docs/report.md"] {
        let p = workspace.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    let mut found = None;
    if let Ok(entries) = fs::read_dir(workspace) {
        for ent in entries.flatten() {
            let p = ent.path();
            if p.extension().and_then(|e| e.to_str()) == Some("md")
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n != "README.md" && n != "components.md")
            {
                found = Some(p);
                break;
            }
        }
    }
    found
}

fn find_workbook_json(workspace: &Path) -> Option<PathBuf> {
    for name in ["workbook.json", "docs/workbook.json"] {
        let p = workspace.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn try_auto_render_pptx(pptx: &Path, workspace: &Path) -> Option<String> {
    let start = if pptx.is_file() {
        pptx.parent().unwrap_or(workspace)
    } else {
        workspace
    };
    let repo = find_repo_root(start)?;
    let script = repo.join("scripts/office/render_pptx_evidence.py");
    let evidence = workspace.join("evidence");
    let output = Command::new("python3")
        .arg(&script)
        .arg(pptx)
        .arg(&evidence)
        .output()
        .ok()?;
    if output.status.success() {
        None
    } else {
        Some(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

fn collect_slide_thumbs(workspace: &Path) -> Vec<String> {
    let mut thumbs = Vec::new();
    let evidence = workspace.join("evidence");
    let dirs = [workspace.to_path_buf(), evidence];
    for dir in &dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(dir) {
            for ent in entries.flatten() {
                let p = ent.path();
                let name = p
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if (name.contains("slide") || name.contains("thumb") || name.contains("render"))
                    && (name.ends_with(".png") || name.ends_with(".jpg"))
                {
                    thumbs.push(p.display().to_string());
                }
            }
        }
    }
    thumbs
}

fn pptx_slide_xml_stats(path: &Path) -> Result<(usize, usize, usize, usize, usize), String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    let mut slide_count = 0usize;
    let mut shape_total = 0usize;
    let mut pic_total = 0usize;
    let mut text_len = 0usize;
    let mut raster_slides = 0usize;
    for i in 0..archive.len() {
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };
        let name = entry.name().to_string();
        if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
            slide_count += 1;
            let mut xml = String::new();
            if entry.read_to_string(&mut xml).is_ok() {
                let sp = xml.matches("<p:sp").count();
                let pic = xml.matches("<p:pic").count();
                shape_total += sp;
                pic_total += pic;
                let mut slide_text = 0usize;
                for part in xml.split("<a:t").skip(1) {
                    if let Some(end) = part.find("</a:t>") {
                        slide_text += part[..end].trim_start_matches('>').trim().chars().count();
                    }
                }
                text_len += slide_text;
                if pic >= 1 && sp <= 2 && slide_text < 20 {
                    raster_slides += 1;
                }
            }
        }
    }
    if slide_count == 0 {
        return Err("no slides found".into());
    }
    Ok((slide_count, shape_total, pic_total, text_len, raster_slides))
}

const MIN_SHAPES_PER_SLIDE: f64 = 5.0;

fn which(bin: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

pub struct DocxOpenValidator;

#[async_trait::async_trait]
impl ArtifactValidator for DocxOpenValidator {
    fn id(&self) -> &'static str {
        "office.docx_open"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".docx"]) else {
            return missing(requirement, self, "DOCX path missing");
        };
        match zip_openable(Path::new(&path)) {
            Ok(()) => passed(requirement, self, &path),
            Err(e) => failed(requirement, self, &path, "docx_unreadable", e),
        }
    }
}

pub struct DocxStructureValidator;

#[async_trait::async_trait]
impl ArtifactValidator for DocxStructureValidator {
    fn id(&self) -> &'static str {
        "office.docx_structure"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".docx"]) else {
            return missing(requirement, self, "DOCX path missing");
        };
        let p = Path::new(&path);
        if !zip_contains(p, "[Content_Types].xml").unwrap_or(false) {
            return failed(
                requirement,
                self,
                &path,
                "docx_missing_content_types",
                "[Content_Types].xml missing".into(),
            );
        }
        if !zip_contains(p, "word/document.xml").unwrap_or(false) {
            return failed(
                requirement,
                self,
                &path,
                "docx_missing_document",
                "word/document.xml missing".into(),
            );
        }
        match zip_read_string(p, "word/document.xml") {
            Ok(xml) => {
                let mut diagnostics = Vec::new();
                if !xml.contains("w:p") {
                    diagnostics.push("no paragraphs (w:p) found".into());
                }
                let has_heading = xml.contains("Heading1")
                    || xml.contains("Heading 1")
                    || xml.contains("heading 1")
                    || xml.contains("Title")
                    || xml.contains("w:outlineLvl");
                if !has_heading {
                    diagnostics.push(
                        "missing heading hierarchy (expected Heading1/Title or outlineLvl)".into(),
                    );
                }
                let lower = xml.to_ascii_lowercase();
                let has_decision_or_action =
                    lower.contains("decision:") || lower.contains("action:");
                if !has_decision_or_action {
                    diagnostics.push(
                        "missing Decision: or Action: ownership lines in document body".into(),
                    );
                }
                for bad in ["tbd", "lorem ipsum", "placeholder", "competitor x"] {
                    if lower.contains(bad) {
                        diagnostics.push(format!("forbidden placeholder `{bad}`"));
                    }
                }
                if diagnostics.is_empty() {
                    passed(requirement, self, &path)
                } else {
                    VerificationResult {
                        gate_id: requirement.id.clone(),
                        validator_id: self.id().into(),
                        validator_version: self.version().into(),
                        outcome: VerificationOutcome::TaskFailed,
                        severity: requirement.severity,
                        artifact_path: Some(path),
                        artifact_hash: None,
                        error_code: Some("docx_structure".into()),
                        diagnostics,
                        evidence_paths: vec![],
                    }
                }
            }
            Err(e) => failed(requirement, self, &path, "docx_read_failed", e),
        }
    }
}

pub struct DocxCommercialValidator;

#[async_trait::async_trait]
impl ArtifactValidator for DocxCommercialValidator {
    fn id(&self) -> &'static str {
        "office.docx_commercial"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".docx"]) else {
            return missing(requirement, self, "DOCX path missing");
        };
        let p = Path::new(&path);
        let mut diagnostics = Vec::new();
        let has_header_part = zip_contains(p, "word/header1.xml").unwrap_or(false)
            || zip_contains(p, "word/header2.xml").unwrap_or(false);
        let has_footer_part = zip_contains(p, "word/footer1.xml").unwrap_or(false)
            || zip_contains(p, "word/footer2.xml").unwrap_or(false);
        if !has_header_part && !has_footer_part {
            if let Ok(xml) = zip_read_string(p, "word/document.xml") {
                let lower = xml.to_ascii_lowercase();
                if !lower.contains("headerreference") && !lower.contains("footerreference") {
                    diagnostics.push(
                        "missing page header/footer (expected word/header*.xml or references)"
                            .into(),
                    );
                }
            } else {
                diagnostics.push("missing page header/footer parts".into());
            }
        }
        if let Ok(styles) = zip_read_string(p, "word/styles.xml") {
            let lower = styles.to_ascii_lowercase();
            if !lower.contains("heading") && !lower.contains("title") {
                diagnostics.push("styles.xml missing Heading/Title style definitions".into());
            }
        } else {
            diagnostics.push("word/styles.xml missing".into());
        }
        if diagnostics.is_empty() {
            passed(requirement, self, &path)
        } else {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::TaskFailed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: Some("docx_commercial".into()),
                diagnostics,
                evidence_paths: vec![],
            }
        }
    }
}

pub struct DocxClassificationValidator;

#[async_trait::async_trait]
impl ArtifactValidator for DocxClassificationValidator {
    fn id(&self) -> &'static str {
        "office.docx_classification"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".docx"]) else {
            return missing(requirement, self, "DOCX path missing");
        };
        let p = Path::new(&path);
        let mut blob = String::new();
        if let Ok(doc) = zip_read_string(p, "word/document.xml") {
            blob.push_str(&doc);
        }
        if let Ok(hdr) = zip_read_string(p, "word/header1.xml") {
            blob.push_str(&hdr);
        }
        let lower = blob.to_ascii_lowercase();
        let hits = [
            "密级",
            "内部",
            "秘密",
            "机密",
            "classification",
            "confidential",
        ];
        if hits.iter().any(|k| lower.contains(&k.to_ascii_lowercase())) {
            passed(requirement, self, &path)
        } else {
            failed(
                requirement,
                self,
                &path,
                "docx_classification_missing",
                "expected classification label (密级/内部/Classification) in header or body".into(),
            )
        }
    }
}

pub struct PptxOpenValidator;

#[async_trait::async_trait]
impl ArtifactValidator for PptxOpenValidator {
    fn id(&self) -> &'static str {
        "office.pptx_open"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".pptx"]) else {
            return missing(requirement, self, "PPTX path missing");
        };
        match zip_openable(Path::new(&path)) {
            Ok(()) => passed(requirement, self, &path),
            Err(e) => failed(requirement, self, &path, "pptx_unreadable", e),
        }
    }
}

pub struct PptxStructureValidator;

#[async_trait::async_trait]
impl ArtifactValidator for PptxStructureValidator {
    fn id(&self) -> &'static str {
        "office.pptx_structure"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".pptx"]) else {
            return missing(requirement, self, "PPTX path missing");
        };
        let p = Path::new(&path);
        if !zip_contains(p, "ppt/presentation.xml").unwrap_or(false) {
            return failed(
                requirement,
                self,
                &path,
                "pptx_missing_presentation",
                "ppt/presentation.xml missing".into(),
            );
        }
        let file = match fs::File::open(p) {
            Ok(f) => f,
            Err(e) => return failed(requirement, self, &path, "pptx_open_failed", e.to_string()),
        };
        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => return failed(requirement, self, &path, "pptx_zip_failed", e.to_string()),
        };
        let mut slide_count = 0usize;
        for i in 0..archive.len() {
            if let Ok(name) = archive.by_index(i).map(|e| e.name().to_string()) {
                if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                    slide_count += 1;
                }
            }
        }
        if slide_count < 2 {
            return failed(
                requirement,
                self,
                &path,
                "pptx_too_few_slides",
                format!("expected ≥2 slides, found {slide_count}"),
            );
        }
        let mut diagnostics = Vec::new();
        for i in 1..=slide_count.min(8) {
            let member = format!("ppt/slides/slide{i}.xml");
            if let Ok(xml) = zip_read_string(p, &member) {
                let lower = xml.to_ascii_lowercase();
                for bad in ["tbd", "lorem ipsum", "placeholder", "competitor x"] {
                    if lower.contains(bad) {
                        diagnostics.push(format!("slide{i}: forbidden placeholder `{bad}`"));
                    }
                }
            }
        }
        if diagnostics.is_empty() {
            passed(requirement, self, &path)
        } else {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::TaskFailed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: Some("pptx_structure".into()),
                diagnostics,
                evidence_paths: vec![],
            }
        }
    }
}

pub struct PptxEditableValidator;

#[async_trait::async_trait]
impl ArtifactValidator for PptxEditableValidator {
    fn id(&self) -> &'static str {
        "office.pptx_editable"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".pptx"]) else {
            return missing(requirement, self, "PPTX path missing");
        };
        match pptx_slide_xml_stats(Path::new(&path)) {
            Ok((slides, _shapes, pics, text_len, raster_slides)) => {
                let mut diagnostics = Vec::new();
                if text_len < 120 {
                    diagnostics.push(format!(
                        "insufficient editable text in deck (a:t chars={text_len}, need ≥120)"
                    ));
                }
                if pics >= slides && slides >= 2 && raster_slides >= slides.saturating_sub(1) {
                    diagnostics.push(
                        "raster-only deck detected (full-slide blip images) — use presentation-commercial-delivery"
                            .into(),
                    );
                }
                if diagnostics.is_empty() {
                    passed(requirement, self, &path)
                } else {
                    VerificationResult {
                        gate_id: requirement.id.clone(),
                        validator_id: self.id().into(),
                        validator_version: self.version().into(),
                        outcome: VerificationOutcome::TaskFailed,
                        severity: requirement.severity,
                        artifact_path: Some(path),
                        artifact_hash: None,
                        error_code: Some("pptx_not_editable".into()),
                        diagnostics,
                        evidence_paths: vec![],
                    }
                }
            }
            Err(e) => failed(requirement, self, &path, "pptx_editable_failed", e),
        }
    }
}

pub struct PptxDisclaimerValidator;

#[async_trait::async_trait]
impl ArtifactValidator for PptxDisclaimerValidator {
    fn id(&self) -> &'static str {
        "office.pptx_disclaimer"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".pptx"]) else {
            return missing(requirement, self, "PPTX path missing");
        };
        match pptx_slide_xml_stats(Path::new(&path)) {
            Ok((_slides, _shapes, _pics, text_len, _raster)) => {
                let p = Path::new(&path);
                let file = match fs::File::open(p) {
                    Ok(f) => f,
                    Err(e) => {
                        return failed(requirement, self, &path, "pptx_open_failed", e.to_string())
                    }
                };
                let mut archive = match zip::ZipArchive::new(file) {
                    Ok(a) => a,
                    Err(e) => {
                        return failed(requirement, self, &path, "pptx_zip_failed", e.to_string())
                    }
                };
                let mut combined = String::new();
                for i in 0..archive.len() {
                    let Ok(mut entry) = archive.by_index(i) else {
                        continue;
                    };
                    let name = entry.name().to_string();
                    if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
                        let mut xml = String::new();
                        if entry.read_to_string(&mut xml).is_ok() {
                            combined.push_str(&xml);
                        }
                    }
                }
                let lower = combined.to_ascii_lowercase();
                let hits = [
                    "免责声明",
                    "风险提示",
                    "disclaimer",
                    "not medical advice",
                    "investment risk",
                ];
                if text_len >= 80 && hits.iter().any(|k| lower.contains(*k)) {
                    passed(requirement, self, &path)
                } else {
                    failed(
                        requirement,
                        self,
                        &path,
                        "pptx_disclaimer_missing",
                        "expected disclaimer slide/text (免责声明/disclaimer) for regulated industry deck".into(),
                    )
                }
            }
            Err(e) => failed(requirement, self, &path, "pptx_disclaimer_failed", e),
        }
    }
}

pub struct PptxDensityValidator;

#[async_trait::async_trait]
impl ArtifactValidator for PptxDensityValidator {
    fn id(&self) -> &'static str {
        "office.pptx_density"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".pptx"]) else {
            return missing(requirement, self, "PPTX path missing");
        };
        match pptx_slide_xml_stats(Path::new(&path)) {
            Ok((slides, shapes, pics, _text, _raster)) => {
                let avg = shapes as f64 / slides as f64;
                if avg + f64::EPSILON >= MIN_SHAPES_PER_SLIDE {
                    passed(requirement, self, &path)
                } else {
                    failed(
                        requirement,
                        self,
                        &path,
                        "pptx_low_density",
                        format!(
                            "expected ≥{MIN_SHAPES_PER_SLIDE:.0} native shapes/slide; {slides} slides, {shapes} sp, {pics} pic, avg {avg:.1}"
                        ),
                    )
                }
            }
            Err(e) => failed(requirement, self, &path, "pptx_density_failed", e),
        }
    }
}

pub struct XlsxOpenValidator;

#[async_trait::async_trait]
impl ArtifactValidator for XlsxOpenValidator {
    fn id(&self) -> &'static str {
        "office.xlsx_open"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".xlsx"]) else {
            return missing(requirement, self, "XLSX path missing");
        };
        match zip_openable(Path::new(&path)) {
            Ok(()) => passed(requirement, self, &path),
            Err(e) => failed(requirement, self, &path, "xlsx_unreadable", e),
        }
    }
}

pub struct XlsxStructureValidator;

#[async_trait::async_trait]
impl ArtifactValidator for XlsxStructureValidator {
    fn id(&self) -> &'static str {
        "office.xlsx_structure"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".xlsx"]) else {
            return missing(requirement, self, "XLSX path missing");
        };
        let p = Path::new(&path);
        if !zip_contains(p, "[Content_Types].xml").unwrap_or(false) {
            return failed(
                requirement,
                self,
                &path,
                "xlsx_missing_content_types",
                "[Content_Types].xml missing".into(),
            );
        }
        if !zip_contains(p, "xl/workbook.xml").unwrap_or(false) {
            return failed(
                requirement,
                self,
                &path,
                "xlsx_missing_workbook",
                "xl/workbook.xml missing".into(),
            );
        }
        let file = match fs::File::open(p) {
            Ok(f) => f,
            Err(e) => return failed(requirement, self, &path, "xlsx_open_failed", e.to_string()),
        };
        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => return failed(requirement, self, &path, "xlsx_zip_failed", e.to_string()),
        };
        let mut sheet_count = 0usize;
        let mut row_hint = 0usize;
        let mut blob = String::new();
        for i in 0..archive.len() {
            let Ok(mut entry) = archive.by_index(i) else {
                continue;
            };
            let name = entry.name().to_string();
            if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
                sheet_count += 1;
                let mut buf = String::new();
                if entry.read_to_string(&mut buf).is_ok() {
                    row_hint += buf.matches("<row").count();
                    blob.push_str(&buf);
                }
            } else if name == "xl/sharedStrings.xml" {
                let mut buf = String::new();
                if entry.read_to_string(&mut buf).is_ok() {
                    blob.push_str(&buf);
                }
            }
        }
        let mut diagnostics = Vec::new();
        if sheet_count < 1 {
            diagnostics.push("expected ≥1 worksheet under xl/worksheets/".into());
        }
        if row_hint < 2 {
            diagnostics.push(format!(
                "expected ≥2 data rows (header + body); found row markers ≈{row_hint}"
            ));
        }
        let lower = blob.to_ascii_lowercase();
        for bad in ["tbd", "lorem ipsum", "placeholder"] {
            if lower.contains(bad) {
                diagnostics.push(format!("forbidden placeholder `{bad}`"));
            }
        }
        if diagnostics.is_empty() {
            passed(requirement, self, &path)
        } else {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::TaskFailed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: Some("xlsx_structure".into()),
                diagnostics,
                evidence_paths: vec![],
            }
        }
    }
}

pub struct XlsxStyleValidator;

#[async_trait::async_trait]
impl ArtifactValidator for XlsxStyleValidator {
    fn id(&self) -> &'static str {
        "office.xlsx_style"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        _context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".xlsx"]) else {
            return missing(requirement, self, "XLSX path missing");
        };
        let p = Path::new(&path);
        let file = match fs::File::open(p) {
            Ok(f) => f,
            Err(e) => return failed(requirement, self, &path, "xlsx_open_failed", e.to_string()),
        };
        let mut archive = match zip::ZipArchive::new(file) {
            Ok(a) => a,
            Err(e) => return failed(requirement, self, &path, "xlsx_zip_failed", e.to_string()),
        };
        let mut sheet_count = 0usize;
        let mut styles_blob = String::new();
        for i in 0..archive.len() {
            let Ok(mut entry) = archive.by_index(i) else {
                continue;
            };
            let name = entry.name().to_string();
            if name.starts_with("xl/worksheets/sheet") && name.ends_with(".xml") {
                sheet_count += 1;
            } else if name == "xl/styles.xml" {
                let mut buf = String::new();
                if entry.read_to_string(&mut buf).is_ok() {
                    styles_blob = buf;
                }
            }
        }
        let mut diagnostics = Vec::new();
        if sheet_count < 3 {
            diagnostics.push(format!(
                "expected ≥3 worksheets for commercial workbook; found {sheet_count}"
            ));
        }
        let styles_lower = styles_blob.to_ascii_lowercase();
        let has_brand_fill = styles_lower.contains("patternfill")
            || styles_lower.contains("fgcolor")
            || styles_lower.contains("fill");
        if !has_brand_fill {
            diagnostics.push(
                "xl/styles.xml missing header fill / patternFill (brand table styling)".into(),
            );
        }
        if diagnostics.is_empty() {
            passed(requirement, self, &path)
        } else {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::TaskFailed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: Some("xlsx_style".into()),
                diagnostics,
                evidence_paths: vec![],
            }
        }
    }
}

/// Prefer pre-rendered thumbnails; auto-render via repo script when missing.
pub struct PptxRenderThumbsValidator;

#[async_trait::async_trait]
impl ArtifactValidator for PptxRenderThumbsValidator {
    fn id(&self) -> &'static str {
        "office.pptx_render_thumbs"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        context: &ValidationContext,
    ) -> VerificationResult {
        let Some(path) = first_path(candidates, &[".pptx"]) else {
            return missing(requirement, self, "PPTX path missing");
        };
        let pptx_path = Path::new(&path);
        let mut thumbs = collect_slide_thumbs(&context.workspace);
        let mut render_note = None;
        if thumbs.is_empty() {
            render_note = try_auto_render_pptx(pptx_path, &context.workspace);
            thumbs = collect_slide_thumbs(&context.workspace);
        }
        if !thumbs.is_empty() {
            return VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::Passed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: None,
                diagnostics: render_note
                    .map(|n| vec![format!("auto-render note: {n}")])
                    .unwrap_or_default(),
                evidence_paths: thumbs,
            };
        }
        if soffice_available() {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::TaskFailed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: Some("pptx_thumbs_missing".into()),
                diagnostics: vec![
                    "LibreOffice present but no slide thumbnail PNGs after auto-render".into(),
                ],
                evidence_paths: vec![],
            }
        } else {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::EnvironmentFailed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: Some("render_unavailable".into()),
                diagnostics: vec![
                    "no render thumbs — install LibreOffice (soffice) or pillow for PIL fallback"
                        .into(),
                ],
                evidence_paths: vec![],
            }
        }
    }
}

fn missing(
    requirement: &GateRequirement,
    v: &dyn ArtifactValidator,
    msg: &str,
) -> VerificationResult {
    VerificationResult {
        gate_id: requirement.id.clone(),
        validator_id: v.id().into(),
        validator_version: v.version().into(),
        outcome: VerificationOutcome::TaskFailed,
        severity: requirement.severity,
        artifact_path: None,
        artifact_hash: None,
        error_code: Some("missing_artifact".into()),
        diagnostics: vec![msg.into()],
        evidence_paths: vec![],
    }
}

/// Reuse `scripts/office/validate_slide_html.py` (same as anycode-ppt skill).
pub struct SlideHtmlValidateValidator;

#[async_trait::async_trait]
impl ArtifactValidator for SlideHtmlValidateValidator {
    fn id(&self) -> &'static str {
        "office.slide_html_validate"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        context: &ValidationContext,
    ) -> VerificationResult {
        let Some(dir) = find_slides_dir(&context.workspace, candidates) else {
            return missing(requirement, self, "slides HTML directory missing");
        };
        match run_office_script(
            &context.workspace,
            "scripts/office/validate_slide_html.py",
            &[dir.to_str().unwrap_or("."), "anycode-ppt"],
        ) {
            Ok(()) => passed(requirement, self, &dir.display().to_string()),
            Err((code, msg)) if code == "repo_root_missing" || code == "script_missing" => {
                VerificationResult {
                    gate_id: requirement.id.clone(),
                    validator_id: self.id().into(),
                    validator_version: self.version().into(),
                    outcome: VerificationOutcome::EnvironmentFailed,
                    severity: requirement.severity,
                    artifact_path: Some(dir.display().to_string()),
                    artifact_hash: None,
                    error_code: Some(code),
                    diagnostics: vec![msg],
                    evidence_paths: vec![],
                }
            }
            Err((code, msg)) => failed(requirement, self, &dir.display().to_string(), &code, msg),
        }
    }
}

/// Reuse `scripts/office/validate_report_md.py` (same as anycode-docx skill).
pub struct ReportMdValidateValidator;

#[async_trait::async_trait]
impl ArtifactValidator for ReportMdValidateValidator {
    fn id(&self) -> &'static str {
        "office.report_md_validate"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        context: &ValidationContext,
    ) -> VerificationResult {
        let md = first_path(candidates, &[".md"])
            .map(PathBuf::from)
            .or_else(|| find_report_md(&context.workspace));
        let Some(md) = md else {
            // Preview/docx-only delivery without MD source → skip as Info pass.
            return VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::Passed,
                severity: requirement.severity,
                artifact_path: None,
                artifact_hash: None,
                error_code: None,
                diagnostics: vec!["no report.md in workspace — skipped MD validate".into()],
                evidence_paths: vec![],
            };
        };
        match run_office_script(
            &context.workspace,
            "scripts/office/validate_report_md.py",
            &[md.to_str().unwrap_or("report.md"), "anycode-docx"],
        ) {
            Ok(()) => passed(requirement, self, &md.display().to_string()),
            Err((code, msg)) if code == "repo_root_missing" || code == "script_missing" => {
                VerificationResult {
                    gate_id: requirement.id.clone(),
                    validator_id: self.id().into(),
                    validator_version: self.version().into(),
                    outcome: VerificationOutcome::EnvironmentFailed,
                    severity: requirement.severity,
                    artifact_path: Some(md.display().to_string()),
                    artifact_hash: None,
                    error_code: Some(code),
                    diagnostics: vec![msg],
                    evidence_paths: vec![],
                }
            }
            Err((code, msg)) => failed(requirement, self, &md.display().to_string(), &code, msg),
        }
    }
}

/// Reuse `scripts/office/validate_workbook.py` (same as anycode-xlsx skill).
pub struct WorkbookValidateValidator;

#[async_trait::async_trait]
impl ArtifactValidator for WorkbookValidateValidator {
    fn id(&self) -> &'static str {
        "office.workbook_validate"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        candidates: &[Artifact],
        context: &ValidationContext,
    ) -> VerificationResult {
        let src = first_path(candidates, &[".json", ".csv"])
            .map(PathBuf::from)
            .or_else(|| find_workbook_json(&context.workspace));
        let Some(src) = src else {
            return VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::Passed,
                severity: requirement.severity,
                artifact_path: None,
                artifact_hash: None,
                error_code: None,
                diagnostics: vec!["no workbook.json — skipped source validate".into()],
                evidence_paths: vec![],
            };
        };
        match run_office_script(
            &context.workspace,
            "scripts/office/validate_workbook.py",
            &[src.to_str().unwrap_or("workbook.json"), "anycode-xlsx"],
        ) {
            Ok(()) => passed(requirement, self, &src.display().to_string()),
            Err((code, msg)) if code == "repo_root_missing" || code == "script_missing" => {
                VerificationResult {
                    gate_id: requirement.id.clone(),
                    validator_id: self.id().into(),
                    validator_version: self.version().into(),
                    outcome: VerificationOutcome::EnvironmentFailed,
                    severity: requirement.severity,
                    artifact_path: Some(src.display().to_string()),
                    artifact_hash: None,
                    error_code: Some(code),
                    diagnostics: vec![msg],
                    evidence_paths: vec![],
                }
            }
            Err((code, msg)) => failed(requirement, self, &src.display().to_string(), &code, msg),
        }
    }
}

fn passed(
    requirement: &GateRequirement,
    v: &dyn ArtifactValidator,
    path: &str,
) -> VerificationResult {
    VerificationResult {
        gate_id: requirement.id.clone(),
        validator_id: v.id().into(),
        validator_version: v.version().into(),
        outcome: VerificationOutcome::Passed,
        severity: requirement.severity,
        artifact_path: Some(path.to_string()),
        artifact_hash: None,
        error_code: None,
        diagnostics: vec![],
        evidence_paths: vec![path.to_string()],
    }
}

fn failed(
    requirement: &GateRequirement,
    v: &dyn ArtifactValidator,
    path: &str,
    code: &str,
    msg: String,
) -> VerificationResult {
    VerificationResult {
        gate_id: requirement.id.clone(),
        validator_id: v.id().into(),
        validator_version: v.version().into(),
        outcome: VerificationOutcome::TaskFailed,
        severity: requirement.severity,
        artifact_path: Some(path.to_string()),
        artifact_hash: None,
        error_code: Some(code.into()),
        diagnostics: vec![msg],
        evidence_paths: vec![],
    }
}
