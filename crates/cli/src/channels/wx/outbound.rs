//! Proactive outbound WeChat text and media (dashboard tool, cron, send-test).

use super::cron_notify::{load_cron_notify_target, save_cron_notify_target, CronNotifyTarget};
use super::deliverable::send_deliverable_path;
use super::fields::{i64_snake_camel, msgs_array, str_snake_camel, sync_buf_from_response};
use super::ilink::{is_stale_wechat_session, load_sync_buf, save_sync_buf, WeChatApi, WxSender};
use super::outbound_queue::wechat_outbound_log_path;
use super::send_media::resolve_media_path;
use super::store::{load_latest_account, wcc_data_dir};
use anycode_tools::WeChatMediaDelivery;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const SCHEDULER_LOCK: &str = ".anycode/tasks/scheduler.lock";

fn bridge_scheduler_active() -> bool {
    dirs::home_dir()
        .map(|h| h.join(SCHEDULER_LOCK).is_file())
        .unwrap_or(false)
}

/// Poll `getupdates` once (when the bridge is not running) to refresh `context_token`.
pub async fn refresh_target_from_updates(
    data_root: &Path,
    api: &WeChatApi,
    hint_user_id: &str,
) -> Result<Option<CronNotifyTarget>> {
    if bridge_scheduler_active() {
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
        });
    }
    if let Some(ref t) = latest {
        save_cron_notify_target(data_root, t).context("save refreshed cron_notify_target")?;
    }
    Ok(latest)
}

struct OutboundPrepared {
    data_root: PathBuf,
    sender: WxSender,
    target: CronNotifyTarget,
    target_refreshed: bool,
}

async fn prepare_outbound_sender(data_dir: Option<PathBuf>) -> Result<OutboundPrepared> {
    let data_root = wcc_data_dir(data_dir);
    let account = load_latest_account(&data_root).context("load wechat account")?;
    let Some(mut target) = load_cron_notify_target(&data_root) else {
        anyhow::bail!(
            "wechat cron_notify_target.json missing; send any message to the WeChat bot first"
        );
    };
    let api = Arc::new(WeChatApi::new(
        account.bot_token.clone(),
        account.base_url.clone(),
    ));
    let mut target_refreshed = false;
    if let Some(fresh) =
        refresh_target_from_updates(&data_root, api.as_ref(), &target.from_user_id).await?
    {
        target = fresh;
        target_refreshed = true;
    }
    let outbound_log = wechat_outbound_log_path(&data_root);
    let sender = WxSender::new(api, account.account_id).with_outbound_log(outbound_log);
    Ok(OutboundPrepared {
        data_root,
        sender,
        target,
        target_refreshed,
    })
}

fn stale_session_hint(e: &anyhow::Error) -> &'static str {
    if is_stale_wechat_session(e) {
        "; ask the user to send a new message to the WeChat bot, then retry"
    } else {
        ""
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WeChatOutboundSendResult {
    pub ok: bool,
    pub message_chars: usize,
    pub data_dir: String,
    pub target_refreshed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeChatOutboundMediaResult {
    pub ok: bool,
    pub path: String,
    pub file_name: String,
    pub bytes: u64,
    pub delivery: WeChatMediaDelivery,
    pub data_dir: String,
    pub target_refreshed: bool,
}

pub async fn send_wechat_text(
    data_dir: Option<PathBuf>,
    message: String,
) -> Result<WeChatOutboundSendResult> {
    let text = message.trim();
    if text.is_empty() {
        anyhow::bail!("message must not be empty");
    }
    let prepared = prepare_outbound_sender(data_dir).await?;
    if let Err(e) = prepared
        .sender
        .send_text(
            &prepared.target.from_user_id,
            &prepared.target.context_token,
            text,
        )
        .await
    {
        let hint = stale_session_hint(&e);
        return Err(e).context(format!("wechat send failed{hint}"));
    }
    Ok(WeChatOutboundSendResult {
        ok: true,
        message_chars: text.chars().count(),
        data_dir: prepared.data_root.display().to_string(),
        target_refreshed: prepared.target_refreshed,
    })
}

pub async fn send_wechat_media(
    data_dir: Option<PathBuf>,
    path_token: &str,
    caption: Option<&str>,
) -> Result<WeChatOutboundMediaResult> {
    let token = path_token.trim();
    if token.is_empty() {
        anyhow::bail!("path must not be empty");
    }
    let Some(path) = resolve_media_path(token, None) else {
        anyhow::bail!("file not found or not a regular file: {token}");
    };
    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    let bytes = std::fs::metadata(&path)
        .with_context(|| format!("read metadata for {}", path.display()))?
        .len();

    let prepared = prepare_outbound_sender(data_dir).await?;
    let delivery = send_deliverable_path(
        &prepared.sender,
        &prepared.target.from_user_id,
        &prepared.target.context_token,
        &path,
        caption,
    )
    .await
    .map_err(|e| {
        let hint = stale_session_hint(&e);
        e.context(format!("wechat media send failed{hint}"))
    })?;

    Ok(WeChatOutboundMediaResult {
        ok: true,
        path: path.display().to_string(),
        file_name,
        bytes,
        delivery,
        data_dir: prepared.data_root.display().to_string(),
        target_refreshed: prepared.target_refreshed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn media_path_token_must_not_be_empty() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(send_wechat_media(None, "  ", None))
            .unwrap_err();
        assert!(err.to_string().contains("path must not be empty"));
    }

    #[test]
    fn media_path_must_resolve_to_file() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(send_wechat_media(
                None,
                "/nonexistent/anycode-test-file.pdf",
                None,
            ))
            .unwrap_err();
        assert!(err.to_string().contains("file not found"));
    }
}
