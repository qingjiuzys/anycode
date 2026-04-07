//! 全屏 TUI（ratatui + crossterm）。
//!
//! 子模块：`styles` / `util` / `input` / `transcript` / `chrome` / `approval` / `run`。

mod approval;
mod chrome;
pub(crate) mod input;
mod pet;
mod run;
pub(crate) mod styles;
mod transcript;
pub(crate) mod util;

pub use run::run_tui;
