//! Runtime compact policy knobs.
//!
//! 与 Claude Code 2.1.218 `services/compact` 对齐的环境变量：
//! - `DISABLE_AUTO_COMPACT`：完全禁用自动压缩。
//! - `CLAUDE_CODE_AUTO_COMPACT_WINDOW`：显式 token 窗口；**设置后优先**于 pct/ratio/hard。
//!   字符串「CLAUDE_CODE_AUTO_COMPACT_WINDOW is set and takes precedence. Unset it to
//!   change this setting.」来自二进制。
//! - `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`：0–100 的百分比覆盖默认触发比例。
//! - `CLAUDE_AFTER_LAST_COMPACT`：距上次压缩至少积累的 token 数，不足则暂缓。
//! - `CLAUDE_CODE_COLD_COMPACT`：冷压缩（久未压缩的会话直接整段压缩）。
//! - `CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP`：禁用压缩前 skip 优化。

#[derive(Debug, Clone)]
pub struct CompactPolicy {
    pub trigger_ratio: f32,
    pub hard_token_threshold: u32,
    pub suppress_follow_up_questions: bool,
    /// `CLAUDE_AUTOCOMPACT_PCT_OVERRIDE`（0.0..=1.0，解析自 0–100）。
    pub pct_override: Option<f32>,
    /// `CLAUDE_CODE_AUTO_COMPACT_WINDOW`：设置后优先于 pct/ratio/hard。
    pub auto_compact_window: Option<u32>,
    /// `CLAUDE_AFTER_LAST_COMPACT`：距上次压缩至少积累的 token 数。
    pub after_last_compact: Option<u32>,
    /// `CLAUDE_CODE_COLD_COMPACT`。
    pub cold_compact: bool,
    /// `DISABLE_AUTO_COMPACT`。
    pub disable: bool,
    /// `CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP`。
    pub disable_precompact_skip: bool,
}

impl Default for CompactPolicy {
    fn default() -> Self {
        Self {
            trigger_ratio: 0.88,
            hard_token_threshold: 0,
            suppress_follow_up_questions: true,
            pct_override: None,
            auto_compact_window: None,
            after_last_compact: None,
            cold_compact: false,
            disable: false,
            disable_precompact_skip: false,
        }
    }
}

fn parse_pct(value: &str) -> Option<f32> {
    let pct = value.trim().parse::<f32>().ok()?;
    if (0.0..=100.0).contains(&pct) {
        Some(pct / 100.0)
    } else {
        None
    }
}

fn parse_positive_tokens(value: &str) -> Option<u32> {
    let t = value.trim().parse::<u32>().ok()?;
    (t > 0).then_some(t)
}

impl CompactPolicy {
    /// 从 Claude Code 对齐的环境变量解析策略（可叠加在显式构造之上）。
    pub fn from_env() -> Self {
        Self::default().with_env()
    }

    pub fn apply_env(&mut self) {
        self.apply_env_with(|name| std::env::var(name).ok());
    }

    /// builder 风格：在显式构造基础上叠加 Claude Code 对齐的环境变量。
    pub fn with_env(mut self) -> Self {
        self.apply_env();
        self
    }

    /// 核心解析逻辑：环境读取函数可注入，便于测试隔离（不依赖进程全局 env）。
    fn apply_env_with<F: Fn(&str) -> Option<String>>(&mut self, get: F) {
        if let Some(v) = get("CLAUDE_AUTOCOMPACT_PCT_OVERRIDE") {
            if let Some(pct) = parse_pct(&v) {
                self.pct_override = Some(pct);
            }
        }
        if let Some(v) = get("CLAUDE_CODE_AUTO_COMPACT_WINDOW") {
            if let Some(win) = parse_positive_tokens(&v) {
                self.auto_compact_window = Some(win);
            }
        }
        if let Some(v) = get("CLAUDE_AFTER_LAST_COMPACT") {
            if let Some(min_since) = parse_positive_tokens(&v) {
                self.after_last_compact = Some(min_since);
            }
        }
        let flag = |name: &str| -> bool {
            get(name)
                .map(|v| {
                    let t = v.trim().to_ascii_lowercase();
                    !(t.is_empty()
                        || t == "0"
                        || t == "false"
                        || t == "no"
                        || t == "off"
                        || t == "disabled")
                })
                .unwrap_or(false)
        };
        self.cold_compact = flag("CLAUDE_CODE_COLD_COMPACT");
        self.disable = flag("DISABLE_AUTO_COMPACT");
        self.disable_precompact_skip = flag("CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP");
    }

    /// 触发阈值：**min(设置, 模型窗口)**。
    /// - 设置了 `auto_compact_window` 时它优先；
    /// - 否则 `pct_override` 覆盖 `trigger_ratio`；
    /// - `hard_token_threshold > 0` 时作为绝对下限；
    /// - 任一设置都不超过 `context_window_tokens`（模型窗口封顶）。
    pub fn threshold_tokens(&self, context_window_tokens: u32) -> u32 {
        if context_window_tokens == 0 {
            return 0;
        }
        if let Some(win) = self.auto_compact_window {
            return win.min(context_window_tokens);
        }
        let ratio = self
            .pct_override
            .unwrap_or(self.trigger_ratio)
            .clamp(0.0, 1.0);
        let by_ratio = ((context_window_tokens as f32) * ratio) as u32;
        let threshold = if self.hard_token_threshold > 0 {
            self.hard_token_threshold
                .min(by_ratio.max(self.hard_token_threshold))
        } else {
            by_ratio
        };
        threshold.min(context_window_tokens)
    }

    pub fn should_compact(&self, context_window_tokens: u32, last_input_tokens: u32) -> bool {
        if self.disable || context_window_tokens == 0 {
            return false;
        }
        let threshold = self.threshold_tokens(context_window_tokens);
        threshold > 0 && last_input_tokens >= threshold
    }

    /// 带 `CLAUDE_AFTER_LAST_COMPACT` 语义：即使达到阈值，距上次压缩不足也不压缩。
    pub fn should_compact_since(
        &self,
        context_window_tokens: u32,
        last_input_tokens: u32,
        tokens_since_last_compact: u32,
    ) -> bool {
        if !self.should_compact(context_window_tokens, last_input_tokens) {
            return false;
        }
        if let Some(min_since) = self.after_last_compact {
            if tokens_since_last_compact < min_since {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn policy_from_env(env: &[(&str, &str)]) -> CompactPolicy {
        let map: HashMap<&str, &str> = env.iter().copied().collect();
        let mut p = CompactPolicy::default();
        p.apply_env_with(|name| map.get(name).map(|v| (*v).to_string()));
        p
    }

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
    fn custom_trigger_ratio_applies() {
        let p = CompactPolicy {
            trigger_ratio: 0.5,
            ..Default::default()
        };
        assert!(!p.should_compact(10_000, 4_999));
        assert!(p.should_compact(10_000, 5_000));
    }

    #[test]
    fn hard_token_threshold_zero_uses_ratio() {
        let p = CompactPolicy {
            hard_token_threshold: 0,
            trigger_ratio: 0.88,
            ..Default::default()
        };
        assert!(!p.should_compact(100, 87));
        assert!(p.should_compact(100, 88));
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

    #[test]
    fn window_takes_precedence_and_is_capped_by_context_window() {
        // min(设置, 模型窗口)：窗口 200k、模型 100k → 100k。
        let p = CompactPolicy {
            auto_compact_window: Some(200_000),
            ..Default::default()
        };
        assert_eq!(p.threshold_tokens(100_000), 100_000);
        assert!(p.should_compact(100_000, 100_000));
        assert!(!p.should_compact(100_000, 99_999));
        // 窗口 50k、模型 200k → 50k（窗口优先）。
        let p2 = CompactPolicy {
            auto_compact_window: Some(50_000),
            hard_token_threshold: 120_000,
            trigger_ratio: 0.9,
            ..Default::default()
        };
        assert_eq!(p2.threshold_tokens(200_000), 50_000);
        assert!(p2.should_compact(200_000, 50_000));
        assert!(!p2.should_compact(200_000, 49_999));
    }

    #[test]
    fn pct_override_replaces_trigger_ratio() {
        let p = CompactPolicy {
            pct_override: Some(0.5),
            trigger_ratio: 0.88,
            ..Default::default()
        };
        assert_eq!(p.threshold_tokens(10_000), 5_000);
        assert!(!p.should_compact(10_000, 4_999));
        assert!(p.should_compact(10_000, 5_000));
    }

    #[test]
    fn disable_auto_compact_never_compacts() {
        let p = CompactPolicy {
            disable: true,
            ..Default::default()
        };
        assert!(!p.should_compact(100_000, 100_000));
        assert!(!p.should_compact_since(100_000, 100_000, 1_000_000));
    }

    #[test]
    fn after_last_compact_defers_until_min_accumulated() {
        let p = CompactPolicy {
            after_last_compact: Some(20_000),
            ..Default::default()
        };
        assert!(!p.should_compact_since(100_000, 90_000, 19_999));
        assert!(p.should_compact_since(100_000, 90_000, 20_000));
    }

    #[test]
    fn env_parsing_matches_claude_variable_names() {
        let p = policy_from_env(&[
            ("CLAUDE_AUTOCOMPACT_PCT_OVERRIDE", "50"),
            ("CLAUDE_CODE_AUTO_COMPACT_WINDOW", "64000"),
            ("CLAUDE_AFTER_LAST_COMPACT", "10000"),
            ("CLAUDE_CODE_COLD_COMPACT", "1"),
            ("DISABLE_AUTO_COMPACT", "0"),
        ]);
        assert_eq!(p.pct_override, Some(0.5));
        assert_eq!(p.auto_compact_window, Some(64_000));
        assert_eq!(p.after_last_compact, Some(10_000));
        assert!(p.cold_compact);
        assert!(!p.disable);
        // 窗口优先：即使 pct=0.5 且模型窗口 200k，阈值取 min(64k, 200k)。
        assert_eq!(p.threshold_tokens(200_000), 64_000);
    }

    #[test]
    fn invalid_env_values_are_ignored() {
        let p = policy_from_env(&[
            ("CLAUDE_AUTOCOMPACT_PCT_OVERRIDE", "150"),
            ("CLAUDE_CODE_AUTO_COMPACT_WINDOW", "0"),
        ]);
        assert_eq!(p.pct_override, None);
        assert_eq!(p.auto_compact_window, None);
    }

    #[test]
    fn cold_compact_and_precompact_skip_flags_parse() {
        let p = policy_from_env(&[
            ("CLAUDE_CODE_COLD_COMPACT", "false"),
            ("CLAUDE_CODE_DISABLE_PRECOMPACT_SKIP", "1"),
        ]);
        assert!(!p.cold_compact);
        assert!(p.disable_precompact_skip);
    }
}
