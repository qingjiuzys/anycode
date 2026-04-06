//! Agent 工具循环常量（`execute_task` 与 `execute_turn_from_messages` 共用，避免魔数分叉）。

pub const MAX_AGENT_TURNS: usize = 8;
pub const MAX_TOOL_CALLS_TOTAL: usize = 32;
pub const TOOL_RESULT_MAX_BYTES: usize = 8 * 1024;
pub const TOOL_INPUT_LOG_MAX_BYTES: usize = 2 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_are_positive() {
        assert!(MAX_AGENT_TURNS > 0);
        assert!(MAX_TOOL_CALLS_TOTAL >= MAX_AGENT_TURNS);
        assert!(TOOL_RESULT_MAX_BYTES >= TOOL_INPUT_LOG_MAX_BYTES);
    }
}
