//! Embedded [`AgentRuntime`] bootstrap for dashboard web chat.

use anycode_agent::AgentRuntime;
use anycode_bootstrap::{
    initialize_runtime, load_project_enabled_skills, MemoryAttachMode, RuntimeHosts,
    WorkbenchAskUserQuestionHost,
};
use anycode_config::load_config_for_session;
use anycode_core::DiskTaskOutput;
use anycode_dashboard_ipc::cancel_ipc;
use anycode_tools::AskUserQuestionHost;
use std::path::PathBuf;
use std::sync::Arc;

/// Build a shared in-process runtime for dashboard chat and task triggers.
pub async fn build_embedded_runtime(
    _disk_output: Option<DiskTaskOutput>,
    project_root: &std::path::Path,
) -> anyhow::Result<Arc<AgentRuntime>> {
    let mut config = load_config_for_session(None, anycode_config::env_ignore_approval()).await?;
    anycode_config::apply_project_overlays(&mut config, project_root);
    let project_enabled = load_project_enabled_skills(project_root).await;
    let hosts = RuntimeHosts {
        ask_user_question_host: Some(
            Arc::new(WorkbenchAskUserQuestionHost::new()) as Arc<dyn AskUserQuestionHost>
        ),
        ..Default::default()
    };
    initialize_runtime(
        &config,
        hosts,
        MemoryAttachMode::Shared,
        project_enabled,
        Some(project_root),
    )
    .await
}

#[must_use]
pub fn embedded_chat_enabled() -> bool {
    true
}

pub fn web_chat_log_dir() -> PathBuf {
    cancel_ipc::dashboard_state_dir().join("web-chat")
}
