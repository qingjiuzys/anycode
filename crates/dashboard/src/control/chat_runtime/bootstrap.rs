//! Embedded [`AgentRuntime`] bootstrap for dashboard web chat.

use anycode_agent::AgentRuntime;
use anycode_bootstrap::{
    initialize_runtime, load_project_enabled_skills, MemoryAttachMode, RuntimeHosts,
};
use anycode_config::load_config_for_session;
use anycode_core::DiskTaskOutput;
use anycode_dashboard_ipc::cancel_ipc;
use std::path::PathBuf;
use std::sync::Arc;

/// Build a shared in-process runtime for dashboard chat and task triggers.
pub async fn build_embedded_runtime(
    _disk_output: Option<DiskTaskOutput>,
) -> anyhow::Result<Arc<AgentRuntime>> {
    let config = load_config_for_session(None, false).await?;
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let project_enabled = load_project_enabled_skills(&cwd).await;
    let hosts = RuntimeHosts {
        dialoguer_on_tty: false,
        ..Default::default()
    };
    initialize_runtime(&config, hosts, MemoryAttachMode::Shared, project_enabled).await
}

#[must_use]
pub fn embedded_chat_enabled() -> bool {
    true
}

pub fn web_chat_log_dir() -> PathBuf {
    cancel_ipc::dashboard_state_dir().join("web-chat")
}
