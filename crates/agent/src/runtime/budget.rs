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
    last_decision: BudgetDecision,
}

impl RuntimeBudgetState {
    pub(super) fn new(budget: TaskBudget) -> Option<Self> {
        (!budget.is_empty()).then(|| Self {
            budget: normalize_budget(budget),
            started_at: Instant::now(),
            consumed_tokens: 0,
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
    }

    pub(super) fn evaluate(&self) -> BudgetDecision {
        if let Some(max) = self.budget.max_duration_secs {
            if self.started_at.elapsed() >= Duration::from_secs(max) {
                return BudgetDecision::Stop;
            }
        }
        let Some(total) = self.budget.token_budget_total else {
            return BudgetDecision::Continue;
        };
        if total == 0 {
            return BudgetDecision::Stop;
        }
        let ratio = self.consumed_tokens as f32 / total as f32;
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
            "[{event}] consumed_tokens={} token_budget={} elapsed_secs={} max_duration_secs={}",
            state.consumed_tokens,
            token_budget,
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
}
