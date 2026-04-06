//! 全屏 TUI（ratatui + crossterm）。
//!
//! 子模块：`styles` / `util` / `input` / `transcript` / `chrome` / `approval` / `run`。

mod approval;
mod chrome;
mod input;
mod pet;
mod run;
mod styles;
mod transcript;
mod util;

pub use run::run_tui;
