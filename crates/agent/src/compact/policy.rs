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
        if self.hard_token_threshold > 0 {
            return last_input_tokens >= self.hard_token_threshold;
        }
        (last_input_tokens as f32) >= (context_window_tokens as f32 * self.trigger_ratio)
    }
}
