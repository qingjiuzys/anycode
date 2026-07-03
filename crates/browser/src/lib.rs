//! Native Chromium CDP browser for anycode agents and workbench.

pub mod ax_tree;
pub mod chromium;
pub mod error;
pub mod policy;
pub mod service;
pub mod session_actor;
pub mod snapshot;
pub mod types;

pub use chromium::{chromium_doctor_message, resolve_chromium_executable};
pub use error::{BrowserError, BrowserResult};
pub use service::BrowserService;
pub use types::{
    BrowserScreenshot, BrowserSessionInfo, BrowserSnapshot, BrowserState, BrowserTabInfo,
    LockHolder, ScreencastFrame,
};
