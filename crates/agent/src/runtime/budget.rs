use super::limits::{
    DEFAULT_BUDGET_DEGRADE_RATIO, DEFAULT_BUDGET_HARD_STOP_RATIO, DEFAULT_BUDGET_WARN_RATIO,
};
use super::logging::RunLogger;
use anycode_core::prelude::*;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BudgetDecision {
    Continue,
    Warn,
    Degrade,
    Stop,
}

#[derive(Debug, Clone)]
pub(super) struct RuntimeBudgetState {
    pub(super) budget: TaskBudget,
    started_at: Instant,
    pub(super) consumed_tokens: u32,
    pub(super) consumed_cost_usd: f64,
    last_decision: BudgetDecision,
}

impl RuntimeBudgetState {
    pub(super) fn new(budget: TaskBudget) -> Option<Self> {
        (!budget.is_empty()).then(|| Self {
            budget: normalize_budget(budget),
            started_at: Instant::now(),
            consumed_tokens: 0,
            consumed_cost_usd: 0.0,
            last_decision: BudgetDecision::Continue,
        })
    }

    pub(super) fn add_usage(&mut self, usage: &Usage) {
        self.consumed_tokens = self
            .consumed_tokens
            .saturating_add(usage.input_tokens)
            .saturating_add(usage.output_tokens)
            .saturating_add(usage.cache_read_tokens.unwrap_or(0))
            .saturating_add(usage.cache_creation_tokens.unwrap_or(0));
        self.consumed_cost_usd += estimate_usage_cost_usd(usage);
    }

    pub(super) fn evaluate(&self) -> BudgetDecision {
        if let Some(max) = self.budget.max_duration_secs {
            if self.started_at.elapsed() >= Duration::from_secs(max) {
                return BudgetDecision::Stop;
            }
        }
        let token_ratio = self.budget.token_budget_total.map(|total| {
            if total == 0 {
                f32::INFINITY
            } else {
                self.consumed_tokens as f32 / total as f32
            }
        });
        let cost_ratio = self.budget.cost_budget_usd.map(|total| {
            if total <= 0.0 {
                f32::INFINITY
            } else {
                (self.consumed_cost_usd / total) as f32
            }
        });
        let ratio = token_ratio
            .into_iter()
            .chain(cost_ratio)
            .fold(0.0_f32, f32::max);
        if ratio <= 0.0 {
            return BudgetDecision::Continue;
        }
        if ratio >= self.budget.hard_stop_ratio {
            BudgetDecision::Stop
        } else if ratio >= self.budget.degrade_ratio {
            BudgetDecision::Degrade
        } else if ratio >= self.budget.warn_ratio {
            BudgetDecision::Warn
        } else {
            BudgetDecision::Continue
        }
    }

    pub(super) fn should_log(&mut self, decision: BudgetDecision) -> bool {
        if decision == BudgetDecision::Continue || decision == self.last_decision {
            return false;
        }
        self.last_decision = decision;
        true
    }

    fn elapsed_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}

fn estimate_usage_cost_usd(usage: &Usage) -> f64 {
    let input_rate = std::env::var("ANYCODE_BUDGET_INPUT_USD_PER_M")
        .or_else(|_| std::env::var("ANYCODE_DASHBOARD_INPUT_USD_PER_M"))
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(3.0);
    let output_rate = std::env::var("ANYCODE_BUDGET_OUTPUT_USD_PER_M")
        .or_else(|_| std::env::var("ANYCODE_DASHBOARD_OUTPUT_USD_PER_M"))
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(15.0);
    let input_tokens = usage
        .input_tokens
        .saturating_add(usage.cache_read_tokens.unwrap_or(0))
        .saturating_add(usage.cache_creation_tokens.unwrap_or(0));
    (input_tokens as f64 / 1_000_000.0) * input_rate
        + (usage.output_tokens as f64 / 1_000_000.0) * output_rate
}

fn normalize_budget(mut budget: TaskBudget) -> TaskBudget {
    if !(0.0..=1.0).contains(&budget.warn_ratio) {
        budget.warn_ratio = DEFAULT_BUDGET_WARN_RATIO;
    }
    if !(0.0..=1.0).contains(&budget.degrade_ratio) {
        budget.degrade_ratio = DEFAULT_BUDGET_DEGRADE_RATIO;
    }
    if budget.hard_stop_ratio <= 0.0 {
        budget.hard_stop_ratio = DEFAULT_BUDGET_HARD_STOP_RATIO;
    }
    if budget.warn_ratio > budget.degrade_ratio {
        budget.warn_ratio = DEFAULT_BUDGET_WARN_RATIO;
    }
    if budget.degrade_ratio > budget.hard_stop_ratio {
        budget.degrade_ratio = DEFAULT_BUDGET_DEGRADE_RATIO.min(budget.hard_stop_ratio);
    }
    budget
}

pub(super) fn log_budget_event(
    logger: &RunLogger,
    task_id: TaskId,
    state: &RuntimeBudgetState,
    decision: BudgetDecision,
) {
    let event = match decision {
        BudgetDecision::Continue => return,
        BudgetDecision::Warn => "budget_warning",
        BudgetDecision::Degrade => "budget_degrade",
        BudgetDecision::Stop => "budget_exceeded",
    };
    let token_budget = state
        .budget
        .token_budget_total
        .map(|v| v.to_string())
        .unwrap_or_else(|| "<none>".to_string());
    let max_duration = state
        .budget
        .max_duration_secs
        .map(|v| v.to_string())
        .unwrap_or_else(|| "<none>".to_string());
    logger.line(
        task_id,
        &format!(
            "[{event}] consumed_tokens={} token_budget={} consumed_cost_usd={:.6} cost_budget_usd={} elapsed_secs={} max_duration_secs={}",
            state.consumed_tokens,
            token_budget,
            state.consumed_cost_usd,
            state
                .budget
                .cost_budget_usd
                .map(|v| format!("{v:.6}"))
                .unwrap_or_else(|| "<none>".to_string()),
            state.elapsed_secs(),
            max_duration
        ),
    );
}

/// Returns `true` when the budget hard-stop threshold is reached.
pub(super) fn tick_budget(
    logger: &RunLogger,
    task_id: TaskId,
    state: &mut Option<RuntimeBudgetState>,
) -> bool {
    let Some(state) = state.as_mut() else {
        return false;
    };
    let decision = state.evaluate();
    if state.should_log(decision) {
        log_budget_event(logger, task_id, state, decision);
    }
    decision == BudgetDecision::Stop
}

/// Under budget degradation, block nested agents and high-risk shell/MCP tools.
#[must_use]
pub(super) fn tool_blocked_under_degrade(state: &RuntimeBudgetState, tool_name: &str) -> bool {
    if state.evaluate() != BudgetDecision::Degrade {
        return false;
    }
    if tool_name.starts_with("mcp__") {
        return true;
    }
    anycode_core::tool_catalog_entry(tool_name).is_some_and(|entry| {
        matches!(entry.risk_tier, "high" | "critical")
            || matches!(entry.category, "shell" | "mcp" | "orchestration")
    })
}

/// Record LLM usage against an optional budget. Returns `true` when hard stop should end the task.
pub(super) fn record_llm_usage(
    logger: &RunLogger,
    task_id: TaskId,
    state: &mut Option<RuntimeBudgetState>,
    usage: &Usage,
) -> bool {
    let Some(state) = state.as_mut() else {
        return false;
    };
    state.add_usage(usage);
    let decision = state.evaluate();
    if state.should_log(decision) {
        log_budget_event(logger, task_id, state, decision);
    }
    decision == BudgetDecision::Stop
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_warns_and_stops_by_token_ratio() {
        let mut state = RuntimeBudgetState::new(TaskBudget {
            token_budget_total: Some(100),
            ..TaskBudget::default()
        })
        .unwrap();
        state.add_usage(&Usage {
            input_tokens: 40,
            output_tokens: 10,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        });
        assert_eq!(state.evaluate(), BudgetDecision::Warn);
        state.add_usage(&Usage {
            input_tokens: 50,
            output_tokens: 0,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        });
        assert_eq!(state.evaluate(), BudgetDecision::Stop);
    }

    #[test]
    fn normalize_budget_repairs_bad_ratios() {
        let budget = normalize_budget(TaskBudget {
            token_budget_total: Some(100),
            warn_ratio: 2.0,
            degrade_ratio: 3.0,
            hard_stop_ratio: 0.0,
            ..TaskBudget::default()
        });
        assert!(budget.warn_ratio <= budget.degrade_ratio);
        assert!(budget.degrade_ratio <= budget.hard_stop_ratio);
    }

    #[test]
    fn budget_stops_by_cost_ratio() {
        let mut state = RuntimeBudgetState::new(TaskBudget {
            cost_budget_usd: Some(0.000001),
            ..TaskBudget::default()
        })
        .unwrap();
        state.add_usage(&Usage {
            input_tokens: 1_000,
            output_tokens: 1_000,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        });
        assert_eq!(state.evaluate(), BudgetDecision::Stop);
    }
}
