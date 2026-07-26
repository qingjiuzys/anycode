//! Eval baseline comparison: low-model / Codex reference / enhanced low-model.

use serde::{Deserialize, Serialize};

use crate::eval::{EvalResult, EvalScenario, EvalStatus};

/// Which runtime arm produced a result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvalArm {
    LowModel,
    CodexReference,
    LowModelEnhanced,
}

impl EvalArm {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::LowModel => "low_model",
            Self::CodexReference => "codex_reference",
            Self::LowModelEnhanced => "low_model_enhanced",
        }
    }
}

/// Fixed metrics collected per scenario × arm.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct EvalArmMetrics {
    pub scenario_id: String,
    pub arm: String,
    pub passed: bool,
    #[serde(default)]
    pub tool_error_rate: f64,
    #[serde(default)]
    pub rework_turns: u32,
    #[serde(default)]
    pub preference_hit_rate: f64,
    #[serde(default)]
    pub user_repeat_answer_rate: f64,
    #[serde(default)]
    pub total_tokens: u64,
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(default)]
    pub visual_score: Option<f64>,
    #[serde(default)]
    pub human_preference_score: Option<f64>,
    #[serde(default)]
    pub message: String,
}

impl EvalArmMetrics {
    pub fn from_result(arm: EvalArm, result: &EvalResult) -> Self {
        Self {
            scenario_id: result.scenario_id.clone(),
            arm: arm.as_str().to_string(),
            passed: result.status == EvalStatus::Passed,
            message: result.message.clone(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvalSuiteSummary {
    pub suite_id: String,
    pub arms: Vec<EvalArm>,
    pub per_arm_pass_rate: Vec<(String, f64)>,
    /// Enhanced pass rate minus low-model pass rate (positive = improvement).
    pub enhanced_vs_low_delta: f64,
    /// Enhanced pass rate minus Codex reference (negative = still behind Codex).
    pub enhanced_vs_codex_delta: f64,
    pub rows: Vec<EvalArmMetrics>,
}

impl EvalSuiteSummary {
    pub fn compute(suite_id: impl Into<String>, rows: Vec<EvalArmMetrics>) -> Self {
        let arms = vec![
            EvalArm::LowModel,
            EvalArm::CodexReference,
            EvalArm::LowModelEnhanced,
        ];
        let mut per_arm_pass_rate = Vec::new();
        let mut rates = std::collections::HashMap::<&str, f64>::new();
        for arm in &arms {
            let key = arm.as_str();
            let subset: Vec<_> = rows.iter().filter(|r| r.arm == key).collect();
            let rate = if subset.is_empty() {
                0.0
            } else {
                subset.iter().filter(|r| r.passed).count() as f64 / subset.len() as f64
            };
            rates.insert(key, rate);
            per_arm_pass_rate.push((key.to_string(), rate));
        }
        let low = *rates.get(EvalArm::LowModel.as_str()).unwrap_or(&0.0);
        let codex = *rates.get(EvalArm::CodexReference.as_str()).unwrap_or(&0.0);
        let enhanced = *rates
            .get(EvalArm::LowModelEnhanced.as_str())
            .unwrap_or(&0.0);
        Self {
            suite_id: suite_id.into(),
            arms,
            per_arm_pass_rate,
            enhanced_vs_low_delta: enhanced - low,
            enhanced_vs_codex_delta: enhanced - codex,
            rows,
        }
    }

    /// Gate for promoting enhanced path to default runtime.
    pub fn meets_promotion_gate(&self, min_delta_vs_low: f64, max_gap_vs_codex: f64) -> bool {
        self.enhanced_vs_low_delta >= min_delta_vs_low
            && self.enhanced_vs_codex_delta >= -max_gap_vs_codex
    }
}

/// Scenario categories for the real-task baseline corpus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaselineTaskCategory {
    WebDesignImplement,
    CrossFileCoding,
    Refactor,
    Research,
    OfficeDelivery,
    DatabaseSql,
}

impl BaselineTaskCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WebDesignImplement => "web_design_implement",
            Self::CrossFileCoding => "cross_file_coding",
            Self::Refactor => "refactor",
            Self::Research => "research",
            Self::OfficeDelivery => "office_delivery",
            Self::DatabaseSql => "database_sql",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineScenario {
    #[serde(flatten)]
    pub scenario: EvalScenario,
    pub category: BaselineTaskCategory,
    #[serde(default)]
    pub verifier: String,
    #[serde(default)]
    pub requires_screenshot: bool,
}

/// Built-in smoke corpus (machine-checkable stubs; live runs extend via JSON fixtures).
pub fn builtin_baseline_scenarios() -> Vec<BaselineScenario> {
    vec![
        BaselineScenario {
            scenario: EvalScenario {
                id: "baseline.web.landing_dark".into(),
                prompt: "Build a dark-themed landing page with emerald CTA; verify contrast. Output a single self-contained HTML document.".into(),
                agent: Some("general-purpose".into()),
                mode: Some("code".into()),
                expectations: Default::default(),
            },
            category: BaselineTaskCategory::WebDesignImplement,
            verifier: "html_dark_emerald".into(),
            requires_screenshot: true,
        },
        BaselineScenario {
            scenario: EvalScenario {
                id: "baseline.code.rust_helper".into(),
                prompt: "Implement a Rust helper `fn slugify(input: &str) -> String` that lowercases, replaces spaces with '-', and keeps [a-z0-9-]. Include 3 unit tests.".into(),
                agent: Some("general-purpose".into()),
                mode: Some("code".into()),
                expectations: Default::default(),
            },
            category: BaselineTaskCategory::CrossFileCoding,
            verifier: "rust_slugify".into(),
            requires_screenshot: false,
        },
        BaselineScenario {
            scenario: EvalScenario {
                id: "baseline.office.brief_pptx".into(),
                prompt: "Create a short Q3 product briefing deck (5–7 slides) for executives: problem, metric, plan, risks, ask. Return JSON {title, slides:[{title, bullets:[]}]} only.".into(),
                agent: Some("office-writer".into()),
                mode: Some("code".into()),
                expectations: Default::default(),
            },
            category: BaselineTaskCategory::OfficeDelivery,
            verifier: "pptx_json_deck".into(),
            requires_screenshot: false,
        },
        BaselineScenario {
            scenario: EvalScenario {
                id: "baseline.office.docx_report".into(),
                prompt: "Write a weekly ops report as a structured DOCX outline. Return JSON {title, sections:[{heading, level, paragraphs:[]}]} with H1 + H2 hierarchy, summary, and next steps.".into(),
                agent: Some("office-writer".into()),
                mode: Some("code".into()),
                expectations: Default::default(),
            },
            category: BaselineTaskCategory::OfficeDelivery,
            verifier: "docx_json_outline".into(),
            requires_screenshot: false,
        },
        BaselineScenario {
            scenario: EvalScenario {
                id: "baseline.db.saas_schema".into(),
                prompt: "Design a SaaS billing schema (orgs, users, subscriptions, invoices) as PostgreSQL DDL with primary keys, foreign keys, and useful indexes.".into(),
                agent: Some("general-purpose".into()),
                mode: Some("code".into()),
                expectations: Default::default(),
            },
            category: BaselineTaskCategory::DatabaseSql,
            verifier: "sql_ddl_schema".into(),
            requires_screenshot: false,
        },
        BaselineScenario {
            scenario: EvalScenario {
                id: "baseline.sql.unpaid_invoices".into(),
                prompt: "Write a PostgreSQL query: top 10 unpaid invoices in the last 30 days by organization, with org name, invoice id, amount, due_date. Use explicit columns, JOIN, WHERE, ORDER BY, LIMIT.".into(),
                agent: Some("general-purpose".into()),
                mode: Some("code".into()),
                expectations: Default::default(),
            },
            category: BaselineTaskCategory::DatabaseSql,
            verifier: "sql_select_join".into(),
            requires_screenshot: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::EvalResult;

    #[test]
    fn summary_deltas_and_gate() {
        let rows = vec![
            EvalArmMetrics::from_result(EvalArm::LowModel, &EvalResult::passed("a", "ok")),
            EvalArmMetrics {
                passed: false,
                ..EvalArmMetrics::from_result(EvalArm::LowModel, &EvalResult::passed("b", "x"))
            },
            EvalArmMetrics::from_result(EvalArm::CodexReference, &EvalResult::passed("a", "ok")),
            EvalArmMetrics::from_result(EvalArm::CodexReference, &EvalResult::passed("b", "ok")),
            EvalArmMetrics::from_result(EvalArm::LowModelEnhanced, &EvalResult::passed("a", "ok")),
            EvalArmMetrics::from_result(EvalArm::LowModelEnhanced, &EvalResult::passed("b", "ok")),
        ];
        let summary = EvalSuiteSummary::compute("baseline-v1", rows);
        assert!((summary.enhanced_vs_low_delta - 0.5).abs() < 1e-9);
        assert!((summary.enhanced_vs_codex_delta - 0.0).abs() < 1e-9);
        assert!(summary.meets_promotion_gate(0.2, 0.15));
        assert_eq!(builtin_baseline_scenarios().len(), 6);
    }
}
