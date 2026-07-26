//! Independent gate planning and verification reports for Agent completion.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::task_spec::{ExpectedArtifact, TaskFamily};

pub const VERIFICATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GateSeverity {
    P0,
    P1,
    Info,
}

impl GateSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::P0 => "p0",
            Self::P1 => "p1",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GateRequirement {
    pub id: String,
    pub validator_id: String,
    pub artifact_ref: String,
    pub severity: GateSeverity,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GatePlan {
    pub schema_version: u32,
    pub intent_hash: String,
    pub family: Option<TaskFamily>,
    pub requirements: Vec<GateRequirement>,
    #[serde(default)]
    pub extras: HashMap<String, String>,
}

impl GatePlan {
    pub fn empty(intent_hash: impl Into<String>) -> Self {
        Self {
            schema_version: VERIFICATION_SCHEMA_VERSION,
            intent_hash: intent_hash.into(),
            family: None,
            requirements: Vec::new(),
            extras: HashMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.requirements.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationOutcome {
    Passed,
    TaskFailed,
    EnvironmentFailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationResult {
    pub gate_id: String,
    pub validator_id: String,
    pub validator_version: String,
    pub outcome: VerificationOutcome,
    pub severity: GateSeverity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
    #[serde(default)]
    pub diagnostics: Vec<String>,
    #[serde(default)]
    pub evidence_paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VerificationReport {
    pub schema_version: u32,
    pub task_id: String,
    pub gate_plan_hash: String,
    pub results: Vec<VerificationResult>,
}

impl VerificationReport {
    pub fn all_passed(&self) -> bool {
        self.results
            .iter()
            .all(|r| r.outcome == VerificationOutcome::Passed)
    }

    /// P0 gates that must pass for completion (Info gates are observational).
    pub fn all_p0_passed(&self) -> bool {
        self.results
            .iter()
            .all(|r| r.severity != GateSeverity::P0 || r.outcome == VerificationOutcome::Passed)
    }

    pub fn has_environment_failure(&self) -> bool {
        self.results
            .iter()
            .any(|r| r.outcome == VerificationOutcome::EnvironmentFailed)
    }

    pub fn has_blocking_environment_failure(&self) -> bool {
        self.results.iter().any(|r| {
            r.outcome == VerificationOutcome::EnvironmentFailed
                && matches!(r.severity, GateSeverity::P0 | GateSeverity::P1)
        })
    }

    pub fn failed_p0(&self) -> Vec<&VerificationResult> {
        self.results
            .iter()
            .filter(|r| r.severity == GateSeverity::P0 && r.outcome != VerificationOutcome::Passed)
            .collect()
    }

    pub fn repair_diagnostics(&self) -> String {
        let mut lines = vec!["## Verification failed — repair required".to_string()];
        for r in self.results.iter().filter(|r| {
            r.outcome != VerificationOutcome::Passed && r.severity != GateSeverity::Info
        }) {
            lines.push(format!(
                "- gate `{}` ({}/{}): {}",
                r.gate_id,
                r.severity.as_str(),
                match r.outcome {
                    VerificationOutcome::Passed => "passed",
                    VerificationOutcome::TaskFailed => "task_failed",
                    VerificationOutcome::EnvironmentFailed => "environment_failed",
                },
                r.error_code.as_deref().unwrap_or("unspecified")
            ));
            for d in &r.diagnostics {
                lines.push(format!("  - {d}"));
            }
        }
        let passed: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.outcome == VerificationOutcome::Passed)
            .map(|r| r.gate_id.as_str())
            .collect();
        if !passed.is_empty() {
            lines.push(format!("Already passed: {}", passed.join(", ")));
            lines
                .push("Do not regress passed gates. Fix only the failed diagnostics above.".into());
        }
        lines.join("\n")
    }
}

/// Build a deterministic gate plan from expected artifacts (before execution).
pub struct GatePolicy;

impl GatePolicy {
    pub fn plan(
        family: Option<TaskFamily>,
        expected: &[ExpectedArtifact],
        intent_hash: impl Into<String>,
        extras: Option<&HashMap<String, String>>,
    ) -> GatePlan {
        let extras_map = extras.cloned().unwrap_or_default();
        let brand_kit = extras_map
            .get("brand_kit")
            .map(String::as_str)
            .unwrap_or("lingqi");
        let scenario = extras_map.get("scenario").map(String::as_str);
        let mut requirements = Vec::new();
        for art in expected.iter().filter(|a| a.required) {
            requirements.push(GateRequirement {
                id: format!("artifact.{}", art.id),
                validator_id: "artifact.exists".into(),
                artifact_ref: art.id.clone(),
                severity: GateSeverity::P0,
                timeout_ms: 5_000,
            });
            match art.kind.as_str() {
                "html" | "webpage" => {
                    requirements.push(GateRequirement {
                        id: format!("html.parse.{}", art.id),
                        validator_id: "web.html_parse".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P0,
                        timeout_ms: 10_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("html.structure.{}", art.id),
                        validator_id: "web.html_structure".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P0,
                        timeout_ms: 10_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("html.anti_slop.{}", art.id),
                        validator_id: "web.html_anti_slop".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 5_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("html.viewport.{}", art.id),
                        validator_id: "web.html_viewport".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 5_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("web.screenshot.{}", art.id),
                        validator_id: "web.screenshot_evidence".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::Info,
                        timeout_ms: 5_000,
                    });
                }
                "docx" | "document" => {
                    requirements.push(GateRequirement {
                        id: format!("docx.open.{}", art.id),
                        validator_id: "office.docx_open".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P0,
                        timeout_ms: 30_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("docx.structure.{}", art.id),
                        validator_id: "office.docx_structure".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 15_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("docx.commercial.{}", art.id),
                        validator_id: "office.docx_commercial".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 15_000,
                    });
                    if brand_kit == "gov-formal" || scenario == Some("gov-briefing") {
                        requirements.push(GateRequirement {
                            id: format!("docx.classification.{}", art.id),
                            validator_id: "office.docx_classification".into(),
                            artifact_ref: art.id.clone(),
                            severity: GateSeverity::P1,
                            timeout_ms: 15_000,
                        });
                    }
                }
                "pptx" | "presentation" => {
                    requirements.push(GateRequirement {
                        id: format!("pptx.open.{}", art.id),
                        validator_id: "office.pptx_open".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P0,
                        timeout_ms: 30_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("pptx.structure.{}", art.id),
                        validator_id: "office.pptx_structure".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 15_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("pptx.editable.{}", art.id),
                        validator_id: "office.pptx_editable".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 15_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("pptx.density.{}", art.id),
                        validator_id: "office.pptx_density".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 15_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("pptx.render.{}", art.id),
                        validator_id: "office.pptx_render_thumbs".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 60_000,
                    });
                    if scenario == Some("med-aesthetic-proposal")
                        || scenario == Some("finance-quarterly-review")
                    {
                        requirements.push(GateRequirement {
                            id: format!("pptx.disclaimer.{}", art.id),
                            validator_id: "office.pptx_disclaimer".into(),
                            artifact_ref: art.id.clone(),
                            severity: GateSeverity::P1,
                            timeout_ms: 15_000,
                        });
                    }
                }
                "xlsx" | "spreadsheet" | "workbook" => {
                    requirements.push(GateRequirement {
                        id: format!("xlsx.open.{}", art.id),
                        validator_id: "office.xlsx_open".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P0,
                        timeout_ms: 30_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("xlsx.structure.{}", art.id),
                        validator_id: "office.xlsx_structure".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 15_000,
                    });
                    requirements.push(GateRequirement {
                        id: format!("xlsx.style.{}", art.id),
                        validator_id: "office.xlsx_style".into(),
                        artifact_ref: art.id.clone(),
                        severity: GateSeverity::P1,
                        timeout_ms: 15_000,
                    });
                }
                _ => {}
            }
        }
        GatePlan {
            schema_version: VERIFICATION_SCHEMA_VERSION,
            intent_hash: intent_hash.into(),
            family,
            requirements,
            extras: extras_map,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task_spec::ExpectedArtifact;

    #[test]
    fn web_plan_includes_structure_before_artifacts_exist() {
        let expected = vec![ExpectedArtifact {
            id: "landing".into(),
            kind: "html".into(),
            required: true,
            path_globs: vec!["**/*.html".into()],
        }];
        let plan = GatePolicy::plan(Some(TaskFamily::WebDesign), &expected, "hash", None);
        assert!(!plan.is_empty());
        assert!(plan
            .requirements
            .iter()
            .any(|r| r.validator_id == "artifact.exists"));
        assert!(plan
            .requirements
            .iter()
            .any(|r| r.validator_id == "web.html_structure"));
    }

    #[test]
    fn repair_diagnostics_lists_failures() {
        let report = VerificationReport {
            schema_version: 1,
            task_id: "t1".into(),
            gate_plan_hash: "h".into(),
            results: vec![
                VerificationResult {
                    gate_id: "ok".into(),
                    validator_id: "artifact.exists".into(),
                    validator_version: "1".into(),
                    outcome: VerificationOutcome::Passed,
                    severity: GateSeverity::P0,
                    artifact_path: Some("a.html".into()),
                    artifact_hash: None,
                    error_code: None,
                    diagnostics: vec![],
                    evidence_paths: vec![],
                },
                VerificationResult {
                    gate_id: "bad".into(),
                    validator_id: "web.html_structure".into(),
                    validator_version: "1".into(),
                    outcome: VerificationOutcome::TaskFailed,
                    severity: GateSeverity::P0,
                    artifact_path: Some("a.html".into()),
                    artifact_hash: None,
                    error_code: Some("missing_h1".into()),
                    diagnostics: vec!["exactly one H1 required".into()],
                    evidence_paths: vec![],
                },
            ],
        };
        let text = report.repair_diagnostics();
        assert!(text.contains("missing_h1"));
        assert!(text.contains("Already passed: ok"));
    }
}
