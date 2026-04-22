//! 终端交互共享层：样式、输入、会话 transcript、审批与流式终端脚标等。
//! 全屏矩阵主循环已移除；默认 `anycode` 走 [`crate::repl::stream_ratatui`]。

mod terminal_guard;

pub(crate) mod approval;
pub(crate) use approval::{ApprovalDecision, InteractiveApprovalCallback, PendingApproval};
pub(crate) mod hud_text;
pub(crate) mod input;
pub(crate) mod palette;
pub(crate) mod session_persist;
pub(crate) mod status_line;
pub(crate) mod styles;
pub(crate) mod transcript;
pub(crate) mod user_question;
pub(crate) mod util;

pub(crate) use terminal_guard::stream_repl_use_alternate_screen;
pub(crate) use user_question::PendingUserQuestion;
