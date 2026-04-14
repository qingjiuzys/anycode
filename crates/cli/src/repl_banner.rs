//! REPL 首屏欢迎框（stdout），与 stderr 上的 tracing 分离。

use crate::i18n::tr;
use crate::tui::palette;
use console::{style, Color, Style, Term};
use std::path::Path;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};
use uuid::Uuid;

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 无子命令 **非 TTY**（stdio 行式）与 `anycode repl` 使用不同标题与提示行（TTY 无子命令默认走全屏 TUI，不经此欢迎框）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReplWelcomeKind {
    /// 无子命令 + 非 TTY：`run_interactive` stdio 路径。
    EmbeddedMain,
    /// `anycode repl`
    ReplSubcommand,
}

fn border_style() -> Style {
    let (r, g, b) = palette::SECONDARY;
    Style::new().fg(Color::TrueColor(r, g, b))
}

fn pet_face_style() -> Style {
    let (r, g, b) = palette::ASSISTANT_LABEL;
    Style::new().fg(Color::TrueColor(r, g, b)).bold()
}

fn dim_style() -> Style {
    Style::new().dim()
}

/// 按**显示宽度**截断或右侧空格填充到 `max_w`（无 ANSI）。
fn pad_trunc_plain(s: &str, max_w: usize) -> String {
    let mut out = String::new();
    let mut w = 0usize;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > max_w {
            break;
        }
        out.push(ch);
        w += cw;
    }
    if w < max_w {
        out.push_str(&" ".repeat(max_w - w));
    }
    out
}

/// 流式 REPL 欢迎框内单行小标识（与全屏 TUI pet 风格一致，压缩纵向空白）。
const PET_FACE: &str = " ·ᴗ· ";

/// 打印带框欢迎信息；无 TTY 或 `NO_COLOR` 时 `console` 会降级为无 ANSI。
pub(crate) fn print_repl_welcome(
    _working_dir: &Path,
    agent: &str,
    session_skips_approval: bool,
    kind: ReplWelcomeKind,
) {
    let dim = dim_style();

    match kind {
        ReplWelcomeKind::EmbeddedMain => {
            // 嵌入式流式 REPL：不用 ╭─╮ 框，避免与底栏横线叠成满屏分隔线。
            println!(
                "{}",
                dim.apply_to(format!(
                    "anyCode · v{PKG_VERSION} · {agent} {}",
                    PET_FACE.trim()
                ))
            );
            if session_skips_approval {
                println!("{}", dim.apply_to(tr("repl-row-approval")));
            }
        }
        ReplWelcomeKind::ReplSubcommand => {
            let term = Term::stdout();
            let cols = term.size().1 as usize;
            let inner = cols.saturating_sub(4).clamp(28, 92);
            let title = format!(" anyCode REPL · v{PKG_VERSION} ");
            let top = format!("╭{t:─^w$}╮", t = title, w = inner);
            let bottom = format!("╰{}╯", "─".repeat(inner));

            let b = border_style();
            let pet_st = pet_face_style();

            println!("{}", b.apply_to(top));

            let tail1 = format!("  anyCode · v{PKG_VERSION} · {agent}");
            let pw = PET_FACE.width();
            let col = 6usize;
            let gap = col.saturating_sub(pw).max(1);
            let rest_inner = format!("{}{}", " ".repeat(gap), tail1);
            let rest = pad_trunc_plain(&rest_inner, inner.saturating_sub(pw));
            print!("{}", b.apply_to("│"));
            print!("{}", pet_st.apply_to(PET_FACE));
            print!("{}", dim.apply_to(rest));
            println!("{}", b.apply_to("│"));

            if session_skips_approval {
                let row = pad_trunc_plain(&format!("  {}", tr("repl-row-approval")), inner);
                println!(
                    "{}{}{}",
                    b.apply_to("│"),
                    dim.apply_to(row),
                    b.apply_to("│")
                );
            }

            println!("{}", b.apply_to(bottom));
        }
    }
}

pub(crate) fn print_repl_prompt() {
    let (ar, ag, ab) = palette::ACCENT;
    let (sr, sg, sb) = palette::SECONDARY;
    print!(
        "{}{}{}",
        style("▸ ").fg(Color::TrueColor(ar, ag, ab)).bold(),
        style("anycode").fg(Color::TrueColor(sr, sg, sb)).bold(),
        style("> ").bold()
    );
}

pub(crate) fn print_repl_goodbye(session_id: Uuid) {
    println!("{}", style(tr("repl-goodbye")).dim());
    println!(
        "\n{} anycode --resume {}",
        style(tr("tui-exit-resume-print")).dim(),
        style(session_id).dim()
    );
}
