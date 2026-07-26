//! CompletionGuard: independent gate check before declaring a turn/task complete.

use anycode_core::{
    Artifact, ExpectedArtifact, GatePlan, GateSeverity, TaskFamily, VerificationOutcome,
    VerificationReport,
};
use anycode_tools::{ValidationContext, ValidatorRegistry};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardDecision {
    Complete,
    Repair,
    Partial,
    Failed,
}

#[derive(Debug, Clone)]
pub struct GuardOutcome {
    pub decision: GuardDecision,
    pub report: Option<VerificationReport>,
    pub repair_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CompletionGuardPolicy {
    /// When false, guard is a no-op (legacy completion).
    pub enabled: bool,
    pub max_repairs: u32,
    pub enabled_families: Vec<TaskFamily>,
}

impl Default for CompletionGuardPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_repairs: 1,
            // M1: web; M3: office/docx/pptx share OfficeDelivery family.
            enabled_families: vec![TaskFamily::WebDesign, TaskFamily::OfficeDelivery],
        }
    }
}

pub struct CompletionGuard {
    pub registry: Arc<ValidatorRegistry>,
    pub policy: CompletionGuardPolicy,
}

impl CompletionGuard {
    pub fn new(registry: Arc<ValidatorRegistry>, policy: CompletionGuardPolicy) -> Self {
        Self { registry, policy }
    }

    pub fn family_enabled(&self, family: Option<TaskFamily>) -> bool {
        self.policy.enabled
            && family.is_some_and(|f| self.policy.enabled_families.iter().any(|x| *x == f))
    }

    pub async fn evaluate(
        &self,
        task_id: &str,
        family: Option<TaskFamily>,
        plan: Option<&GatePlan>,
        expected: &[ExpectedArtifact],
        artifacts: &[Artifact],
        workspace: &Path,
        repairs_used: u32,
        last_diagnostics: Option<&str>,
    ) -> GuardOutcome {
        if !self.family_enabled(family) {
            return GuardOutcome {
                decision: GuardDecision::Complete,
                report: None,
                repair_message: None,
            };
        }
        let Some(plan) = plan.filter(|p| !p.is_empty()) else {
            return GuardOutcome {
                decision: GuardDecision::Complete,
                report: None,
                repair_message: None,
            };
        };

        let report = self
            .registry
            .run_plan(
                task_id,
                plan,
                expected,
                artifacts,
                &ValidationContext {
                    workspace: workspace.to_path_buf(),
                    extras: plan.extras.clone(),
                },
            )
            .await;

        if report.all_p0_passed()
            && !report.results.iter().any(|r| {
                r.severity == GateSeverity::P1 && r.outcome == VerificationOutcome::TaskFailed
            })
        {
            return GuardOutcome {
                decision: GuardDecision::Complete,
                report: Some(report),
                repair_message: None,
            };
        }

        if report.has_blocking_environment_failure() && report.all_p0_passed() {
            return GuardOutcome {
                decision: GuardDecision::Partial,
                report: Some(report),
                repair_message: None,
            };
        }

        let diagnostics = report.repair_diagnostics();
        if last_diagnostics.is_some_and(|prev| prev == diagnostics) {
            return GuardOutcome {
                decision: GuardDecision::Failed,
                report: Some(report),
                repair_message: Some(
                    "Identical verification diagnostics repeated — stopping repair loop.".into(),
                ),
            };
        }

        let has_p0 = report
            .results
            .iter()
            .any(|r| r.severity == GateSeverity::P0 && r.outcome != VerificationOutcome::Passed);

        if repairs_used < self.policy.max_repairs {
            return GuardOutcome {
                decision: GuardDecision::Repair,
                report: Some(report),
                repair_message: Some(diagnostics),
            };
        }

        GuardOutcome {
            decision: if has_p0 {
                GuardDecision::Failed
            } else {
                GuardDecision::Partial
            },
            report: Some(report),
            repair_message: Some(diagnostics),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::{ExpectedArtifact, GatePolicy};
    use anycode_tools::ValidatorRegistry;

    #[tokio::test]
    async fn requests_repair_when_html_missing() {
        let guard = CompletionGuard::new(
            Arc::new(ValidatorRegistry::new()),
            CompletionGuardPolicy::default(),
        );
        let expected = vec![ExpectedArtifact {
            id: "landing_html".into(),
            kind: "html".into(),
            required: true,
            path_globs: vec!["**/*.html".into()],
        }];
        let plan = GatePolicy::plan(Some(TaskFamily::WebDesign), &expected, "h", None);
        let temp = tempfile::tempdir().unwrap();
        let out = guard
            .evaluate(
                "t1",
                Some(TaskFamily::WebDesign),
                Some(&plan),
                &expected,
                &[],
                temp.path(),
                0,
                None,
            )
            .await;
        assert_eq!(out.decision, GuardDecision::Repair);
        assert!(out.repair_message.unwrap().contains("missing_artifact"));
    }
}
