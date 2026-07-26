//! HTML validators for web delivery gates.

use super::{ArtifactValidator, ValidationContext};
use anycode_core::{
    Artifact, ExpectedArtifact, GateRequirement, VerificationOutcome, VerificationResult,
};
use std::fs;

fn read_first_html(candidates: &[Artifact]) -> Option<(String, String)> {
    for art in candidates {
        let Some(path) = art.path.as_deref() else {
            continue;
        };
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        return Some((path.to_string(), text));
    }
    None
}

pub struct HtmlParseValidator;

#[async_trait::async_trait]
impl ArtifactValidator for HtmlParseValidator {
    fn id(&self) -> &'static str {
        "web.html_parse"
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
        let Some((path, text)) = read_first_html(candidates) else {
            return fail(requirement, self, "missing_artifact", "no HTML candidate");
        };
        let lower = text.to_ascii_lowercase();
        if !lower.contains("<html") || !lower.contains("</html>") {
            return fail(
                requirement,
                self,
                "html_unparseable",
                &format!("{path}: missing <html>…</html>"),
            );
        }
        if text.contains("```") {
            return fail(
                requirement,
                self,
                "markdown_fence",
                &format!("{path}: HTML must not be wrapped in markdown fences"),
            );
        }
        pass(requirement, self, &path)
    }
}

pub struct HtmlStructureValidator;

#[async_trait::async_trait]
impl ArtifactValidator for HtmlStructureValidator {
    fn id(&self) -> &'static str {
        "web.html_structure"
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
        let Some((path, text)) = read_first_html(candidates) else {
            return fail(requirement, self, "missing_artifact", "no HTML candidate");
        };
        let lower = text.to_ascii_lowercase();
        let h1_count = lower.matches("<h1").count();
        let mut diagnostics = Vec::new();
        if h1_count != 1 {
            diagnostics.push(format!("exactly one H1 required, found {h1_count}"));
        }
        if !lower.contains("<a ") {
            diagnostics.push("missing primary/secondary link (anchor)".into());
        }
        let has_contrast_comment = text.contains("contrast") || text.contains("Contrast");
        if !has_contrast_comment {
            diagnostics.push("missing HTML comment with contrast ratio notes".into());
        }
        if diagnostics.is_empty() {
            pass(requirement, self, &path)
        } else {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::TaskFailed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: Some("html_structure".into()),
                diagnostics,
                evidence_paths: vec![],
            }
        }
    }
}

pub struct HtmlAntiSlopValidator;

#[async_trait::async_trait]
impl ArtifactValidator for HtmlAntiSlopValidator {
    fn id(&self) -> &'static str {
        "web.html_anti_slop"
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
        let Some((path, text)) = read_first_html(candidates) else {
            return fail(requirement, self, "missing_artifact", "no HTML candidate");
        };
        let lower = text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();
        for bad in ["purple", "violet", "#7c3aed", "#8b5cf6", "indigo"] {
            if lower.contains(bad) {
                diagnostics.push(format!("forbidden color/token `{bad}`"));
            }
        }
        for font in ["inter", "roboto"] {
            if lower.contains(font) {
                diagnostics.push(format!("prefer distinctive fonts; found `{font}`"));
            }
        }
        let has_anchor = ["terminal", "aside", "panel", "mock", "preview", "grid"]
            .iter()
            .any(|k| lower.contains(k));
        if !has_anchor && lower.matches('{').count() < 8 {
            diagnostics.push(
                "page looks flattened; add a visual anchor (terminal/aside/panel) or richer CSS"
                    .into(),
            );
        }
        if diagnostics.is_empty() {
            pass(requirement, self, &path)
        } else {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::TaskFailed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: Some("html_anti_slop".into()),
                diagnostics,
                evidence_paths: vec![],
            }
        }
    }
}

/// Heuristic viewport / overflow checks without a browser.
pub struct HtmlViewportValidator;

#[async_trait::async_trait]
impl ArtifactValidator for HtmlViewportValidator {
    fn id(&self) -> &'static str {
        "web.html_viewport"
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
        let Some((path, text)) = read_first_html(candidates) else {
            return fail(requirement, self, "missing_artifact", "no HTML candidate");
        };
        let lower = text.to_ascii_lowercase();
        let mut diagnostics = Vec::new();
        if !lower.contains("viewport") {
            diagnostics.push("missing <meta name=\"viewport\">".into());
        }
        if !lower.contains("@media") && !lower.contains("max-width") {
            diagnostics.push("no responsive @media / max-width hints for 375/768/1440".into());
        }
        if lower.contains("overflow-x: hidden") && !lower.contains("overflow-x:auto") {
            // hidden alone often masks overflow bugs — warn
            diagnostics.push(
                "overflow-x:hidden alone can hide horizontal overflow; prefer fixing layout".into(),
            );
        }
        if diagnostics.is_empty() {
            pass(requirement, self, &path)
        } else {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::TaskFailed,
                severity: requirement.severity,
                artifact_path: Some(path),
                artifact_hash: None,
                error_code: Some("html_viewport".into()),
                diagnostics,
                evidence_paths: vec![],
            }
        }
    }
}

/// Require screenshot evidence files when present in workspace; soft fail if none.
pub struct ScreenshotEvidenceValidator;

#[async_trait::async_trait]
impl ArtifactValidator for ScreenshotEvidenceValidator {
    fn id(&self) -> &'static str {
        "web.screenshot_evidence"
    }
    fn version(&self) -> &'static str {
        "1"
    }
    async fn validate(
        &self,
        requirement: &GateRequirement,
        _expected: &ExpectedArtifact,
        _candidates: &[Artifact],
        context: &ValidationContext,
    ) -> VerificationResult {
        let root = &context.workspace;
        let mut found = Vec::new();
        if let Ok(entries) = fs::read_dir(root) {
            for ent in entries.flatten() {
                let path = ent.path();
                let name = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if name.contains("screenshot")
                    || name.contains("viewport")
                    || name.ends_with("-375.png")
                    || name.ends_with("-768.png")
                    || name.ends_with("-1440.png")
                {
                    found.push(path.display().to_string());
                }
            }
        }
        // Also accept nested evidence/
        let evidence = root.join("evidence");
        if evidence.is_dir() {
            if let Ok(entries) = fs::read_dir(&evidence) {
                for ent in entries.flatten() {
                    let path = ent.path();
                    if path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                        e.eq_ignore_ascii_case("png") || e.eq_ignore_ascii_case("jpg")
                    }) {
                        found.push(path.display().to_string());
                    }
                }
            }
        }
        if found.is_empty() {
            // Not a hard task failure for M1 — environment/browser may be unavailable.
            return VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::EnvironmentFailed,
                severity: requirement.severity,
                artifact_path: None,
                artifact_hash: None,
                error_code: Some("screenshot_missing".into()),
                diagnostics: vec![
                    "no screenshot evidence found (expected *screenshot*.png or evidence/*.png)"
                        .into(),
                ],
                evidence_paths: vec![],
            };
        }
        VerificationResult {
            gate_id: requirement.id.clone(),
            validator_id: self.id().into(),
            validator_version: self.version().into(),
            outcome: VerificationOutcome::Passed,
            severity: requirement.severity,
            artifact_path: found.first().cloned(),
            artifact_hash: None,
            error_code: None,
            diagnostics: vec![],
            evidence_paths: found,
        }
    }
}

fn pass(
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

fn fail(
    requirement: &GateRequirement,
    v: &dyn ArtifactValidator,
    code: &str,
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
        error_code: Some(code.into()),
        diagnostics: vec![msg.into()],
        evidence_paths: vec![],
    }
}
