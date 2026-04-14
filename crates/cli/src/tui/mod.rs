//! 全屏 TUI（ratatui + crossterm）。
//!
//! 子模块：`styles` / `util` / `input` / `transcript` / `chrome` / `approval` / `run`。

pub(crate) mod approval;
pub(crate) use approval::{ApprovalDecision, PendingApproval, TuiApprovalCallback};
mod backend;
mod chrome;
pub(crate) mod hud_text;
pub(crate) mod input;
pub(crate) mod palette;
mod pet;
mod run;
pub(crate) mod status_line;
pub(crate) mod styles;
pub(crate) mod transcript;
pub(crate) mod tui_session_persist;
pub(crate) mod user_question;
pub(crate) mod util;

pub use run::run_tui;
pub(crate) use user_question::PendingUserQuestion;
