//! Legacy subprocess web chat hub (retired — dispatch uses embedded `ChatRuntimeHost`).

use crate::db::DashboardDb;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebChatSendResult {
    pub session_id: String,
    pub pid: u32,
    pub log_path: String,
    pub started_at: String,
    pub queued: bool,
}

#[derive(Default, Clone)]
pub struct WebChatHub;

impl WebChatHub {
    pub async fn evict(&self, _session_id: &str) {}

    #[allow(clippy::too_many_arguments)]
    pub async fn send(
        &self,
        _db: DashboardDb,
        _session_id: &str,
        _project_root: &Path,
        _agent: Option<&str>,
        _dashboard_url: &str,
        _prompt: &str,
        _vision_images: Option<&[crate::control::vision_payload::VisionImagePayload]>,
        _text_files: Option<&[crate::control::text_upload::TextFilePayload]>,
        _reply_lang: Option<&str>,
    ) -> Result<WebChatSendResult> {
        bail!("subprocess web chat removed; embedded runtime is always enabled")
    }
}
