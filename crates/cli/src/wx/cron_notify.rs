//! 定时任务触发后向微信会话投递（读取最近对话方 `cron_notify_target.json`）。

use crate::wx::WxSender;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronNotifyTarget {
    pub from_user_id: String,
    pub context_token: String,
}

pub fn cron_notify_path(data_root: &Path) -> PathBuf {
    data_root.join("cron_notify_target.json")
}

pub fn save_cron_notify_target(data_root: &Path, target: &CronNotifyTarget) -> Result<()> {
    let path = cron_notify_path(data_root);
    let j = serde_json::to_string_pretty(target).context("serialize cron_notify_target")?;
    std::fs::write(&path, j).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn load_cron_notify_target(data_root: &Path) -> Option<CronNotifyTarget> {
    let path = cron_notify_path(data_root);
    let raw = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&raw).ok()
}

pub async fn deliver_cron_to_wechat(
    data_root: &Path,
    sender: &WxSender,
    command: &str,
    body: &str,
) {
    let Some(target) = load_cron_notify_target(data_root) else {
        tracing::warn!(
            target: "anycode_scheduler",
            "cron fired but no cron_notify_target.json (no recent WeChat chat)"
        );
        return;
    };
    let text = body.trim();
    let msg = if text.is_empty() {
        format!("⏰ 定时提醒：{command}")
    } else {
        format!("⏰ 定时提醒：{command}\n\n{text}")
    };
    if let Err(e) = sender
        .send_text(&target.from_user_id, &target.context_token, &msg)
        .await
    {
        tracing::warn!(
            target: "anycode_scheduler",
            "cron WeChat deliver failed: {e:#}"
        );
    } else {
        tracing::info!(
            target: "anycode_scheduler",
            user = %target.from_user_id,
            "cron delivered to WeChat"
        );
    }
}
