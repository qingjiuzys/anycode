//! REPL 首屏欢迎框（stdout），与 stderr 上的 tracing 分离。

use crate::i18n::tr;
use console::{style, Style, Term};
use std::path::Path;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");

fn accent() -> Style {
    Style::new().color256(209)
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

/// 路径过长时保留尾部，前缀 `…`。
fn path_row(working_dir: &Path, max_w: usize) -> String {
    let s = working_dir.display().to_string();
    if s.width() <= max_w {
        return pad_trunc_plain(&s, max_w);
    }
    let ell = "…";
    let ew = ell.width();
    let budget = max_w.saturating_sub(ew);
    let mut tail = String::new();
    let mut w = 0usize;
    for ch in s.chars().rev() {
        let cw = ch.width().unwrap_or(0);
        if w + cw > budget {
            break;
        }
        tail.push(ch);
        w += cw;
    }
    let tail: String = tail.chars().rev().collect();
    pad_trunc_plain(&format!("{ell}{tail}"), max_w)
}

fn print_box_line(a: &Style, dim: &Style, inner: usize, plain: &str) {
    println!(
        "{}{}{}",
        a.apply_to("│"),
        dim.apply_to(pad_trunc_plain(plain, inner)),
        a.apply_to("│")
    );
}

/// 打印带框欢迎信息；无 TTY 或 `NO_COLOR` 时 `console` 会降级为无 ANSI。
pub(crate) fn print_repl_welcome(working_dir: &Path, agent: &str, session_skips_approval: bool) {
    let term = Term::stdout();
    let cols = term.size().1 as usize;
    let inner = cols.saturating_sub(4).clamp(36, 92);

    let title = format!(" anyCode REPL · v{PKG_VERSION} ");
    let top = format!("╭{t:─^w$}╮", t = title, w = inner);
    let sep = format!("├{}┤", "─".repeat(inner));
    let bottom = format!("╰{}╯", "─".repeat(inner));

    let a = accent();
    let dim = Style::new().dim();

    println!("{}", a.apply_to(top));
    print_box_line(&a, &dim, inner, &format!("  {}", tr("repl-welcome-line1")));
    print_box_line(&a, &dim, inner, &format!("  {}", tr("repl-welcome-line2")));
    println!("{}", a.apply_to(sep.clone()));
    print_box_line(
        &a,
        &dim,
        inner,
        &format!(
            "  {}  {}",
            tr("repl-row-cwd"),
            path_row(working_dir, inner.saturating_sub(8))
        ),
    );
    print_box_line(
        &a,
        &dim,
        inner,
        &format!("  {} {agent}", tr("repl-row-agent")),
    );
    if session_skips_approval {
        print_box_line(&a, &dim, inner, &format!("  {}", tr("repl-row-approval")));
    }
    println!("{}", a.apply_to(sep));
    print_box_line(&a, &dim, inner, &format!("  {}", tr("repl-row-commands")));
    println!("{}", a.apply_to(bottom));
    println!("{}", style(tr("repl-hint-line")).dim());
    println!("{}", style(tr("repl-hint-debug")).dim());
    println!();
}

pub(crate) fn print_repl_prompt() {
    print!("{}{}", style("anycode").cyan().bold(), style("> ").bold());
}

pub(crate) fn print_repl_goodbye() {
    println!("{}", style(tr("repl-goodbye")).dim());
}
