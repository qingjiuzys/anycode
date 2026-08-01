//! 压缩变体（与 Claude Code 2.1.218 `services/compact` 变体对齐）。
//!
//! 二进制提取语义：
//! - **PreCompact skip**：`SKIP_PRECOMPACT_THRESHOLD`（消息/指纹过少直接跳过压缩）+ `CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP`。
//! - **Cold compact**：`CLAUDE_CODE_COLD_COMPACT`——久未压缩的会话整段压缩（跳过按 gap 的渐进步进）。
//! - **Away summary**：用户离开后回归的 recap（`CLAUDE_CODE_ENABLE_REMOTE_RECAP` / `tengu_harbor_moth`、
//!   `awaySummaryEnabled`、`tengu_sedge_lantern` 门控），专用 prompt：`The user stepped away and is coming back…`。
//! - **Classifier**：预判是否值得压缩的分类器变体（`shouldCompact` 前置判定）。
//! - **ToolUse summaries**：聚合工具调用结果后再压缩（对齐 Claude `toolUseSummary` 意图：减少工具结果体积）。

use anycode_core::prelude::*;

/// 压缩变体枚举。`Precompact` 是常规路径；其余为对齐 Claude 的特定变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompactVariant {
    /// 常规压缩（auto compact / manual compact）。
    Precompact,
    /// `CLAUDE_CODE_COLD_COMPACT`：久未压缩的会话整段压缩。
    Cold,
    /// away summary：用户回归 recap（1 轮、40 词内、无 markdown）。
    Away,
    /// 分类器预判变体：先决定是否值得压缩。
    Classifier,
    /// tool-use summaries：聚合工具结果后再压缩。
    ToolUse,
}

impl CompactVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            CompactVariant::Precompact => "precompact",
            CompactVariant::Cold => "cold",
            CompactVariant::Away => "away",
            CompactVariant::Classifier => "classifier",
            CompactVariant::ToolUse => "tool_use",
        }
    }

    /// 从 Claude Code 对齐的日志/遥测标签解析（`compact_type` / `variant` 字符串）。
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "precompact" | "auto" | "manual" => Some(CompactVariant::Precompact),
            "cold" => Some(CompactVariant::Cold),
            "away" | "away_summary" | "recap" => Some(CompactVariant::Away),
            "classifier" => Some(CompactVariant::Classifier),
            "tool_use" | "tooluse" | "tool-use" => Some(CompactVariant::ToolUse),
            _ => None,
        }
    }
}

/// PreCompact skip 决策：消息/指纹过少时直接跳过压缩（对齐 Claude `SKIP_PRECOMPACT_THRESHOLD`）。
/// `disable_precompact_skip`（`CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP`）为 true 时永不跳过。
pub fn should_skip_precompact(
    message_count: usize,
    threshold: usize,
    disable_precompact_skip: bool,
) -> bool {
    if disable_precompact_skip {
        return false;
    }
    message_count < threshold
}

/// Cold compact 判定：`CLAUDE_CODE_COLD_COMPACT` 开启且距上次压缩足够久（未压缩）时整段压缩。
/// `tokens_since_last_compact` 为 `None` 表示从未压缩过（冷会话）。
pub fn is_cold_compact(
    cold_compact: bool,
    tokens_since_last_compact: Option<u32>,
    min_cold_tokens: u32,
) -> bool {
    if !cold_compact {
        return false;
    }
    match tokens_since_last_compact {
        None => true,
        Some(since) => since >= min_cold_tokens,
    }
}

/// Away summary 门控（对齐二进制 `AWAY_SUMMARY`）：
/// - `CLAUDE_CODE_ENABLE_REMOTE_RECAP` 显式开启（tengu_harbor_moth flag）；
/// - 或 `awaySummaryEnabled` 配置开关（tengu_sedge_lantern 门控后）为 true。
pub fn is_away_summary_enabled(
    enable_remote_recap: Option<bool>,
    away_summary_enabled: Option<bool>,
) -> bool {
    if enable_remote_recap == Some(true) {
        return true;
    }
    away_summary_enabled == Some(true)
}

/// Away recap 专用 prompt（对齐二进制提取文本：40 词内、1–2 句、无 markdown、给下一步动作）。
pub const AWAY_SUMMARY_PROMPT: &str = "The user stepped away and is coming back. \
Recap in under 40 words, 1-2 plain sentences, no markdown. \
Lead with the overall goal and current task, then the one next action. \
Skip root-cause narrative, fix internals, secondary to-dos, and em-dash tangents.";

/// tool-use summaries：聚合「可压缩工具」的 tool_result 为紧凑占位，返回被替换条数。
/// 对齐 Claude `toolUseSummary`：在压缩前把工具结果压缩为摘要形态，减少 LLM 输入体积。
pub fn apply_tool_use_summaries(messages: &mut [Message]) -> usize {
    // 复用 microcompact 的占位逻辑：可压缩工具 + 默认保留 3 条。
    super::microcompact::apply_microcompact(messages, super::microcompact::default_keep_recent())
}

/// 分类器变体的「值得压缩」预判：输入 token 超过阈值、且会话有可摘要内容。
pub fn classifier_should_compact(
    last_input_tokens: u32,
    threshold_tokens: u32,
    has_assistant_content: bool,
) -> bool {
    threshold_tokens > 0 && last_input_tokens >= threshold_tokens && has_assistant_content
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_str_roundtrip() {
        for v in [
            CompactVariant::Precompact,
            CompactVariant::Cold,
            CompactVariant::Away,
            CompactVariant::Classifier,
            CompactVariant::ToolUse,
        ] {
            assert_eq!(CompactVariant::from_str(v.as_str()), Some(v));
        }
        assert_eq!(
            CompactVariant::from_str("away_summary"),
            Some(CompactVariant::Away)
        );
        assert_eq!(CompactVariant::from_str("bogus"), None);
    }

    #[test]
    fn precompact_skip_respects_threshold_and_disable() {
        assert!(should_skip_precompact(1, 2, false));
        assert!(!should_skip_precompact(2, 2, false));
        assert!(!should_skip_precompact(1, 2, true));
    }

    #[test]
    fn cold_compact_only_when_flag_and_stale() {
        assert!(is_cold_compact(true, None, 1_000));
        assert!(is_cold_compact(true, Some(2_000), 1_000));
        assert!(!is_cold_compact(true, Some(500), 1_000));
        assert!(!is_cold_compact(false, None, 1_000));
    }

    #[test]
    fn away_enabled_by_recap_or_config() {
        assert!(is_away_summary_enabled(Some(true), None));
        assert!(is_away_summary_enabled(None, Some(true)));
        assert!(!is_away_summary_enabled(None, None));
        assert!(!is_away_summary_enabled(Some(false), Some(false)));
    }

    #[test]
    fn away_prompt_is_short_and_actionable() {
        assert!(AWAY_SUMMARY_PROMPT.starts_with("The user stepped away and is coming back."));
        assert!(AWAY_SUMMARY_PROMPT.contains("one next action"));
    }

    #[test]
    fn classifier_gates_on_threshold_and_content() {
        assert!(classifier_should_compact(100, 80, true));
        assert!(!classifier_should_compact(100, 80, false));
        assert!(!classifier_should_compact(79, 80, true));
    }
}
