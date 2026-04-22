//! 流式 REPL dock 与底栏复用的脚标文案与 `⎿` 提示轮换。

use crate::i18n::{tr, tr_args};
use anycode_core::TurnTokenUsage;
use fluent_bundle::FluentArgs;

/// Footer ctx 片段：流式 REPL 底栏左列使用（与历史全屏脚标同文案策略）。
pub(crate) fn footer_context_fragment_for_tokens(
    context_window_tokens: u32,
    last_max_input_tokens: u32,
    last_output_tokens: u32,
) -> String {
    let mut base = if context_window_tokens == 0 {
        tr("term-footer-ctx-unknown")
    } else if last_max_input_tokens == 0 {
        let mut a = FluentArgs::new();
        a.set("win", context_window_tokens as i64);
        tr_args("term-footer-ctx-zero", &a)
    } else {
        let pct =
            ((last_max_input_tokens as f64 / context_window_tokens as f64) * 100.0).min(100.0);
        let mut a = FluentArgs::new();
        a.set("pct", (pct.round() as i64).max(0));
        a.set("win", context_window_tokens as i64);
        tr_args("term-footer-ctx-pct", &a)
    };
    if last_output_tokens > 0 {
        let mut a = FluentArgs::new();
        a.set("k", format_tokens_k_thousands(last_output_tokens));
        base.push_str(&tr_args("term-footer-out-tokens", &a));
    }
    base
}

/// 千分位 `k` 展示（HUD 脚标共用，与 Claude「约 Xk tokens」风格一致）。
pub(crate) fn format_tokens_k_thousands(tokens: u32) -> String {
    let tok_k = (tokens as f64) / 1000.0;
    if tok_k >= 100.0 {
        format!("{:.0}", tok_k)
    } else if tok_k >= 10.0 {
        format!("{:.1}", tok_k)
    } else {
        format!("{:.1}", tok_k.max(0.1))
    }
}

/// Prompt 上 `⎿` 提示轮换条数（约每 8 秒换一条）。
pub(crate) const HUD_TIP_COUNT: usize = 6;

const HUD_TIP_IDS: [&str; HUD_TIP_COUNT] = [
    "term-hud-tip-rename",
    "term-hud-tip-resume",
    "term-hud-tip-compact",
    "term-hud-tip-help",
    "term-hud-tip-clear",
    "term-hud-tip-scroll",
];

pub(crate) fn hud_tip_rotated(slot: usize) -> String {
    tr(HUD_TIP_IDS[slot % HUD_TIP_COUNT])
}

/// Prompt HUD 第一行活动文案（与历史全屏栈一致策略）。
pub(crate) fn prompt_hud_activity_text(
    pending_approval: bool,
    executing: bool,
    working_elapsed_secs: Option<u64>,
) -> String {
    if pending_approval {
        tr("term-hud-await-approval")
    } else if executing {
        match working_elapsed_secs {
            Some(s) => {
                let mut a = FluentArgs::new();
                a.set("s", s);
                tr_args("term-hud-executing-secs", &a)
            }
            None => tr("term-hud-executing"),
        }
    } else {
        tr("term-hud-idle")
    }
}

/// Claude 风格：执行结束后短暂显示耗时文案（全屏 TUI 等可复用；流式 REPL 现用 [`prompt_hud_stream_turn_summary_text`]）。
#[allow(dead_code)]
pub(crate) fn prompt_hud_thought_for_text(elapsed_secs: u64) -> String {
    let mut a = FluentArgs::new();
    a.set("s", elapsed_secs.max(1));
    tr_args("term-hud-thought-secs", &a)
}

/// 流式 REPL 回合结束：墙钟耗时 + 本轮聚合 tokens（与全屏脚标同源字段）。
pub(crate) fn prompt_hud_stream_turn_summary_text(
    turn_wall_secs: u64,
    usage: &TurnTokenUsage,
) -> String {
    let mut a = FluentArgs::new();
    a.set("s", turn_wall_secs.max(1) as i64);
    let in_k = format_tokens_k_thousands(usage.max_input_tokens);
    a.set("in_k", in_k);
    if usage.total_output_tokens > 0 {
        a.set(
            "out_k",
            format_tokens_k_thousands(usage.total_output_tokens),
        );
        tr_args("repl-hud-turn-summary-io", &a)
    } else {
        tr_args("repl-hud-turn-summary-in", &a)
    }
}

#[cfg(test)]
mod tests {
    use super::{prompt_hud_stream_turn_summary_text, prompt_hud_thought_for_text};

    #[test]
    fn thought_for_text_uses_elapsed_seconds() {
        let s = prompt_hud_thought_for_text(3);
        assert!(s.contains('3'));
    }

    #[test]
    fn thought_for_text_clamps_zero_to_one() {
        let s = prompt_hud_thought_for_text(0);
        assert!(s.contains('1'));
    }

    #[test]
    fn stream_turn_summary_includes_secs_and_k_tokens() {
        use anycode_core::TurnTokenUsage;
        let u = TurnTokenUsage {
            max_input_tokens: 10_200,
            total_output_tokens: 0,
            total_cache_read_tokens: 0,
            total_cache_creation_tokens: 0,
        };
        let s = prompt_hud_stream_turn_summary_text(10, &u);
        assert!(
            s.contains("10")
                && (s.contains("10.2")
                    || s.contains("ctx")
                    || s.contains("tokens")
                    || s.contains('k')
                    || s.contains("in")),
            "unexpected summary: {s:?}"
        );
    }

    #[test]
    fn stream_turn_summary_shows_output_when_nonzero() {
        use anycode_core::TurnTokenUsage;
        let u = TurnTokenUsage {
            max_input_tokens: 1000,
            total_output_tokens: 500,
            total_cache_read_tokens: 0,
            total_cache_creation_tokens: 0,
        };
        let s = prompt_hud_stream_turn_summary_text(2, &u);
        assert!(
            s.contains('2') && (s.contains("out") || s.contains("0.5")),
            "unexpected summary: {s:?}"
        );
    }
}
