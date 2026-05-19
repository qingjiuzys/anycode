//! Runtime compact policy knobs.

#[derive(Debug, Clone)]
pub struct CompactPolicy {
    pub trigger_ratio: f32,
    pub hard_token_threshold: u32,
    pub suppress_follow_up_questions: bool,
}

impl Default for CompactPolicy {
    fn default() -> Self {
        Self {
            trigger_ratio: 0.88,
            hard_token_threshold: 0,
            suppress_follow_up_questions: true,
        }
    }
}

impl CompactPolicy {
    pub fn should_compact(&self, context_window_tokens: u32, last_input_tokens: u32) -> bool {
        if context_window_tokens == 0 {
            return false;
        }
        if self.hard_token_threshold > 0 {
            return last_input_tokens >= self.hard_token_threshold;
        }
        (last_input_tokens as f32) >= (context_window_tokens as f32 * self.trigger_ratio)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_compact_at_ratio_threshold() {
        let p = CompactPolicy::default();
        assert!(!p.should_compact(100_000, 80_000));
        assert!(!p.should_compact(100_000, 87_999));
        assert!(p.should_compact(100_000, 88_000));
    }

    #[test]
    fn should_not_compact_on_zero_context_window() {
        let p = CompactPolicy::default();
        assert!(!p.should_compact(0, 10_000));
    }

    #[test]
    fn hard_token_threshold_overrides_ratio() {
        let p = CompactPolicy {
            hard_token_threshold: 50_000,
            ..Default::default()
        };
        assert!(p.should_compact(1_000_000, 50_000));
        assert!(!p.should_compact(1_000_000, 49_999));
    }
}
