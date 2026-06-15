//! 定时任务触发后向微信会话投递（读取最近对话方 `cron_notify_target.json`）。

use super::deliverable::{collect_outbound_media_paths, send_outbound_media_paths};
use super::WxSender;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CronNotifyTarget {
    pub from_user_id: String,
    pub context_token: String,
    /// Milliseconds since UNIX epoch when this token was last observed from iLink.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at_ms: Option<u64>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Persist the latest inbound `context_token` for proactive outbound (bridge + cron + tools).
pub fn touch_outbound_context(
    data_root: &Path,
    from_user_id: &str,
    context_token: &str,
) -> Result<()> {
    let token = context_token.trim();
    if token.is_empty() {
        return Ok(());
    }
    save_cron_notify_target(
        data_root,
        &CronNotifyTarget {
            from_user_id: from_user_id.to_string(),
            context_token: token.to_string(),
            updated_at_ms: Some(now_ms()),
        },
    )
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

/// When the bridge is polling, wait briefly for a newer token written from inbound traffic.
pub async fn wait_for_fresher_context_token(
    data_root: &Path,
    previous_token: &str,
    timeout: std::time::Duration,
) -> Option<CronNotifyTarget> {
    use std::time::Instant;
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if let Some(t) = load_cron_notify_target(data_root) {
            if t.context_token != previous_token {
                return Some(t);
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    }
    None
}

/// Poll `getupdates` once (when the bridge is not running) to refresh `context_token`.
pub async fn refresh_target_from_updates(
    data_root: &Path,
    api: &super::ilink::WeChatApi,
    hint_user_id: &str,
) -> anyhow::Result<Option<CronNotifyTarget>> {
    use super::fields::{i64_snake_camel, msgs_array, str_snake_camel, sync_buf_from_response};
    use super::ilink::{load_sync_buf, save_sync_buf};
    use anyhow::Context;

    if super::bridge_lock::wechat_bridge_active(data_root) {
        return Ok(None);
    }
    let buf = load_sync_buf(data_root);
    let resp = api
        .get_updates(if buf.is_empty() { None } else { Some(&buf) })
        .await
        .context("getupdates for outbound token refresh")?;
    if let Some(next) = sync_buf_from_response(&resp) {
        let _ = save_sync_buf(data_root, next);
    }
    let mut latest: Option<CronNotifyTarget> = None;
    for msg in msgs_array(&resp) {
        let msg_type = i64_snake_camel(&msg, "message_type", "messageType").unwrap_or(0);
        if msg_type == 2 {
            continue;
        }
        let from = match str_snake_camel(&msg, "from_user_id", "fromUserId") {
            Some(f) => f,
            None => continue,
        };
        if from != hint_user_id {
            continue;
        }
        let ctx = str_snake_camel(&msg, "context_token", "contextToken")
            .unwrap_or("")
            .trim();
        if ctx.is_empty() {
            continue;
        }
        latest = Some(CronNotifyTarget {
            from_user_id: from.to_string(),
            context_token: ctx.to_string(),
            updated_at_ms: Some(now_ms()),
        });
    }
    if let Some(ref t) = latest {
        touch_outbound_context(data_root, &t.from_user_id, &t.context_token)
            .context("save refreshed cron_notify_target")?;
    }
    Ok(latest)
}

/// After a stale-session send failure, reload disk / wait for bridge / poll getupdates once.
pub async fn try_recover_fresh_context(
    data_root: &Path,
    api: Option<&super::ilink::WeChatApi>,
    current: &CronNotifyTarget,
) -> Option<CronNotifyTarget> {
    if let Some(disk) = load_cron_notify_target(data_root) {
        if disk.context_token != current.context_token {
            return Some(disk);
        }
    }
    if super::bridge_lock::wechat_bridge_active(data_root) {
        return wait_for_fresher_context_token(
            data_root,
            &current.context_token,
            std::time::Duration::from_secs(5),
        )
        .await;
    }
    if let Some(api) = api {
        match refresh_target_from_updates(data_root, api, &current.from_user_id).await {
            Ok(Some(fresh)) if fresh.context_token != current.context_token => return Some(fresh),
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "getupdates token recovery failed after stale session"
                );
            }
        }
    }
    None
}

pub async fn deliver_cron_to_wechat(
    data_root: &Path,
    sender: &WxSender,
    command: &str,
    body: &str,
) {
    let Some(mut target) = load_cron_notify_target(data_root) else {
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
    let mut stale_retried = false;
    loop {
        match sender
            .send_text(&target.from_user_id, &target.context_token, &msg)
            .await
        {
            Ok(()) => break,
            Err(e) if super::ilink::is_stale_wechat_session(&e) && !stale_retried => {
                stale_retried = true;
                if let Some(fresh) =
                    try_recover_fresh_context(data_root, Some(sender.api()), &target).await
                {
                    if fresh.context_token != target.context_token {
                        target = fresh;
                        continue;
                    }
                }
                tracing::warn!(
                    target: "anycode_scheduler",
                    "cron WeChat deliver failed: {e:#}"
                );
                return;
            }
            Err(e) => {
                tracing::warn!(
                    target: "anycode_scheduler",
                    "cron WeChat deliver failed: {e:#}"
                );
                return;
            }
        }
    }
    let combined = format!("{command}\n{text}");
    let paths = collect_outbound_media_paths(&[], &combined, None);
    if !paths.is_empty() {
        let mut stale_retried = false;
        loop {
            match send_outbound_media_paths(
                sender,
                &target.from_user_id,
                &target.context_token,
                &paths,
            )
            .await
            {
                Ok(()) => break,
                Err(e) if super::ilink::is_stale_wechat_session(&e) && !stale_retried => {
                    stale_retried = true;
                    if let Some(fresh) =
                        try_recover_fresh_context(data_root, Some(sender.api()), &target).await
                    {
                        if fresh.context_token != target.context_token {
                            target = fresh;
                            continue;
                        }
                    }
                    tracing::warn!(
                        target: "anycode_scheduler",
                        "cron WeChat deliverable failed: {e:#}"
                    );
                    return;
                }
                Err(e) => {
                    tracing::warn!(
                        target: "anycode_scheduler",
                        "cron WeChat deliverable failed: {e:#}"
                    );
                    return;
                }
            }
        }
    }
    tracing::info!(
        target: "anycode_scheduler",
        user = %target.from_user_id,
        "cron delivered to WeChat"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cron_notify_target_roundtrip_with_updated_at() {
        let dir =
            std::env::temp_dir().join(format!("anycode-wx-cron-target-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        touch_outbound_context(&dir, "user-1", "token-a").unwrap();
        let loaded = load_cron_notify_target(&dir).unwrap();
        assert_eq!(loaded.from_user_id, "user-1");
        assert_eq!(loaded.context_token, "token-a");
        assert!(loaded.updated_at_ms.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
