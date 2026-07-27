//! Independent artifact validators used by CompletionGuard.

mod office;
mod web;

use anycode_core::{
    Artifact, ExpectedArtifact, GatePlan, GateRequirement, VerificationOutcome, VerificationReport,
    VerificationResult,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use office::{
    DocxClassificationValidator, DocxCommercialValidator, DocxOpenValidator,
    DocxStructureValidator, PptxDensityValidator, PptxDisclaimerValidator, PptxEditableValidator,
    PptxOpenValidator, PptxRenderThumbsValidator, PptxStructureValidator,
    ReportMdValidateValidator, SlideHtmlValidateValidator, WorkbookValidateValidator,
    XlsxOpenValidator, XlsxStructureValidator, XlsxStyleValidator,
};
pub use web::{
    HtmlAntiSlopValidator, HtmlParseValidator, HtmlStructureValidator, HtmlViewportValidator,
    ScreenshotEvidenceValidator,
};

#[derive(Debug, Clone, Default)]
pub struct ValidationContext {
    pub workspace: PathBuf,
    pub extras: HashMap<String, String>,
}

#[async_trait::async_trait]
pub trait ArtifactValidator: Send + Sync {
    fn id(&self) -> &'static str;
    fn version(&self) -> &'static str;
    async fn validate(
        &self,
        requirement: &GateRequirement,
        expected: &ExpectedArtifact,
        candidates: &[Artifact],
        context: &ValidationContext,
    ) -> VerificationResult;
}

#[derive(Default)]
pub struct ValidatorRegistry {
    validators: HashMap<String, Arc<dyn ArtifactValidator>>,
}

impl ValidatorRegistry {
    pub fn new() -> Self {
        let mut reg = Self::default();
        reg.register(Arc::new(ArtifactExistsValidator));
        reg.register(Arc::new(HtmlParseValidator));
        reg.register(Arc::new(HtmlStructureValidator));
        reg.register(Arc::new(HtmlAntiSlopValidator));
        reg.register(Arc::new(HtmlViewportValidator));
        reg.register(Arc::new(ScreenshotEvidenceValidator));
        reg.register(Arc::new(DocxOpenValidator));
        reg.register(Arc::new(DocxStructureValidator));
        reg.register(Arc::new(DocxCommercialValidator));
        reg.register(Arc::new(DocxClassificationValidator));
        reg.register(Arc::new(PptxOpenValidator));
        reg.register(Arc::new(PptxStructureValidator));
        reg.register(Arc::new(PptxEditableValidator));
        reg.register(Arc::new(PptxDensityValidator));
        reg.register(Arc::new(PptxDisclaimerValidator));
        reg.register(Arc::new(PptxRenderThumbsValidator));
        reg.register(Arc::new(XlsxOpenValidator));
        reg.register(Arc::new(XlsxStructureValidator));
        reg.register(Arc::new(XlsxStyleValidator));
        reg.register(Arc::new(SlideHtmlValidateValidator));
        reg.register(Arc::new(ReportMdValidateValidator));
        reg.register(Arc::new(WorkbookValidateValidator));
        reg
    }

    pub fn register(&mut self, v: Arc<dyn ArtifactValidator>) {
        self.validators.insert(v.id().to_string(), v);
    }

    pub async fn run_plan(
        &self,
        task_id: &str,
        plan: &GatePlan,
        expected: &[ExpectedArtifact],
        artifacts: &[Artifact],
        context: &ValidationContext,
    ) -> VerificationReport {
        let mut results = Vec::new();
        for req in &plan.requirements {
            let expected_art = expected
                .iter()
                .find(|a| a.id == req.artifact_ref)
                .cloned()
                .unwrap_or(ExpectedArtifact {
                    id: req.artifact_ref.clone(),
                    kind: "file".into(),
                    required: true,
                    path_globs: vec![],
                });
            let candidates = discover_candidates(&expected_art, artifacts, &context.workspace);
            if expected_art.required
                && candidates.is_empty()
                && req.validator_id == "artifact.exists"
            {
                results.push(VerificationResult {
                    gate_id: req.id.clone(),
                    validator_id: req.validator_id.clone(),
                    validator_version: "1".into(),
                    outcome: VerificationOutcome::TaskFailed,
                    severity: req.severity,
                    artifact_path: None,
                    artifact_hash: None,
                    error_code: Some("missing_artifact".into()),
                    diagnostics: vec![format!(
                        "required artifact `{}` not found (globs: {})",
                        expected_art.id,
                        expected_art.path_globs.join(", ")
                    )],
                    evidence_paths: vec![],
                });
                continue;
            }
            let Some(validator) = self.validators.get(&req.validator_id) else {
                results.push(VerificationResult {
                    gate_id: req.id.clone(),
                    validator_id: req.validator_id.clone(),
                    validator_version: "0".into(),
                    outcome: VerificationOutcome::EnvironmentFailed,
                    severity: req.severity,
                    artifact_path: None,
                    artifact_hash: None,
                    error_code: Some("validator_missing".into()),
                    diagnostics: vec![format!("validator `{}` not registered", req.validator_id)],
                    evidence_paths: vec![],
                });
                continue;
            };
            results.push(
                validator
                    .validate(req, &expected_art, &candidates, context)
                    .await,
            );
        }
        VerificationReport {
            schema_version: plan.schema_version,
            task_id: task_id.to_string(),
            gate_plan_hash: plan.intent_hash.clone(),
            results,
        }
    }
}

pub fn discover_candidates(
    expected: &ExpectedArtifact,
    artifacts: &[Artifact],
    workspace: &Path,
) -> Vec<Artifact> {
    let mut out: Vec<Artifact> = artifacts
        .iter()
        .filter(|a| {
            a.path.as_deref().is_some_and(|p| {
                expected.path_globs.iter().any(|g| path_matches_glob(p, g))
                    || a.resolved_kind() == expected.kind
                    || a.name == expected.id
            })
        })
        .cloned()
        .collect();

    let Ok(root) = workspace.canonicalize() else {
        return out;
    };
    for glob_pat in &expected.path_globs {
        if let Some(rel) = glob_pat.strip_prefix("**/") {
            match walkdir_simple(&root, rel) {
                Ok(entries) => {
                    for path in entries {
                        if !path_under_root(&path, &root) {
                            continue;
                        }
                        let display = path.display().to_string();
                        if out
                            .iter()
                            .any(|a| a.path.as_deref() == Some(display.as_str()))
                        {
                            continue;
                        }
                        out.push(Artifact::from_path(path.to_string_lossy().as_ref()));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        target: "anycode_tools",
                        glob = %glob_pat,
                        error = %e,
                        "artifact candidate walk failed"
                    );
                }
            }
        } else {
            let candidate = root.join(glob_pat);
            if let Ok(canon) = candidate.canonicalize() {
                if path_under_root(&canon, &root) && canon.is_file() {
                    let display = canon.display().to_string();
                    if !out
                        .iter()
                        .any(|a| a.path.as_deref() == Some(display.as_str()))
                    {
                        out.push(Artifact::from_path(display.as_str()));
                    }
                }
            }
        }
    }
    out
}

fn path_matches_glob(path: &str, glob: &str) -> bool {
    let name = Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path);
    if let Some(suf) = glob.strip_prefix("**/") {
        if suf.starts_with("*.") {
            return name.ends_with(&suf[1..]);
        }
        return path.ends_with(suf) || name == suf;
    }
    path.ends_with(glob) || name == glob
}

fn path_under_root(path: &Path, root: &Path) -> bool {
    path.starts_with(root)
}

fn walkdir_simple(root: &Path, pattern: &str) -> std::io::Result<Vec<PathBuf>> {
    const MAX_DEPTH: usize = 8;
    const MAX_ENTRIES: usize = 4_096;
    let mut out = Vec::new();
    let mut visited = 0usize;
    let mut stack = vec![(root.to_path_buf(), 0usize)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > MAX_DEPTH || visited >= MAX_ENTRIES {
            break;
        }
        // Tolerate unreadable subdirectories instead of aborting the whole walk.
        let Ok(read) = std::fs::read_dir(&dir) else {
            continue;
        };
        for ent in read.flatten() {
            visited += 1;
            if visited >= MAX_ENTRIES {
                break;
            }
            let path = ent.path();
            if path.is_symlink() {
                continue;
            }
            if path.is_dir() {
                stack.push((path, depth + 1));
            } else if path_matches_glob(path.to_string_lossy().as_ref(), &format!("**/{pattern}")) {
                out.push(path);
            }
        }
    }
    Ok(out)
}

struct ArtifactExistsValidator;

#[async_trait::async_trait]
impl ArtifactValidator for ArtifactExistsValidator {
    fn id(&self) -> &'static str {
        "artifact.exists"
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
        if let Some(art) = candidates.first() {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::Passed,
                severity: requirement.severity,
                artifact_path: art.path.clone(),
                artifact_hash: None,
                error_code: None,
                diagnostics: vec![],
                evidence_paths: art.path.clone().into_iter().collect(),
            }
        } else {
            VerificationResult {
                gate_id: requirement.id.clone(),
                validator_id: self.id().into(),
                validator_version: self.version().into(),
                outcome: VerificationOutcome::TaskFailed,
                severity: requirement.severity,
                artifact_path: None,
                artifact_hash: None,
                error_code: Some("missing_artifact".into()),
                diagnostics: vec!["required artifact missing".into()],
                evidence_paths: vec![],
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::{GatePolicy, TaskFamily};

    #[tokio::test]
    async fn missing_html_fails_exists_gate() {
        let reg = ValidatorRegistry::new();
        let expected = vec![ExpectedArtifact {
            id: "landing_html".into(),
            kind: "html".into(),
            required: true,
            path_globs: vec!["**/*.html".into()],
        }];
        let plan = GatePolicy::plan(Some(TaskFamily::WebDesign), &expected, "h", None);
        let temp = tempfile::tempdir().unwrap();
        let report = reg
            .run_plan(
                "t1",
                &plan,
                &expected,
                &[],
                &ValidationContext {
                    workspace: temp.path().to_path_buf(),
                    extras: HashMap::new(),
                },
            )
            .await;
        assert!(!report.all_passed());
        assert!(report
            .results
            .iter()
            .any(|r| r.error_code.as_deref() == Some("missing_artifact")));
    }
}
