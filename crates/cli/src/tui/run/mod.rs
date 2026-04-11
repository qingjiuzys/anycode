//! TUI 主循环（crossterm 事件 + ratatui 绘制 + agent turn）。

use super::approval::{PendingApproval, TuiApprovalCallback};
use super::input::{InputState, RevSearchState};
use crate::app_config::Config;
use crate::bootstrap::initialize_runtime;
use anycode_security::ApprovalCallback;
use crossterm::event::{self as ct_event};
use ratatui::{backend::CrosstermBackend, Terminal, TerminalOptions, Viewport};
use std::cell::Cell;
use std::io::stdout;
use std::path::PathBuf;
use tokio::sync::mpsc;

mod draw;
mod event;
mod exec_completion;
mod loop_inner;
mod resize_debounce;
mod terminal_guard;
pub use loop_inner::run_tui;
