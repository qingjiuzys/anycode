//! Proactive outbound WeChat text and media (dashboard tool, cron, send-test).

use super::bridge_lock::wechat_bridge_active;
use super::cron_notify::{
    load_cron_notify_target, refresh_target_from_updates, try_recover_fresh_context,
    CronNotifyTarget,
};
use super::deliverable::send_deliverable_path;
use super::ilink::{is_stale_wechat_session, WeChatApi, WxSender};
use super::outbound_queue::wechat_outbound_log_path;
use super::send_media::resolve_media_path;
use super::store::{load_latest_account, wcc_data_dir};
use anycode_tools::WeChatMediaDelivery;
use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

struct OutboundPrepared {
    data_root: PathBuf,
    sender: WxSender,
    target: CronNotifyTarget,
    target_refreshed: bool,
}

async fn resolve_outbound_target(
    data_root: &Path,
    api: &WeChatApi,
    fallback: &CronNotifyTarget,
) -> Result<(CronNotifyTarget, bool)> {
    let mut refreshed = false;
    let mut target = load_cron_notify_target(data_root).unwrap_or_else(|| fallback.clone());

    if wechat_bridge_active(data_root) {
        if target.context_token != fallback.context_token {
            refreshed = true;
        }
        return Ok((target, refreshed));
    }

    match refresh_target_from_updates(data_root, api, &target.from_user_id).await {
        Ok(Some(fresh)) => {
            refreshed = true;
            target = fresh;
        }
        Ok(None) => {}
        Err(e) => {
            tracing::warn!(
                error = %e,
                "getupdates token refresh failed; using saved cron_notify_target"
            );
        }
    }
    Ok((target, refreshed))
}

async fn prepare_outbound_sender(data_dir: Option<PathBuf>) -> Result<OutboundPrepared> {
    let data_root = wcc_data_dir(data_dir);
    let account = load_latest_account(&data_root).context("load wechat account")?;
    let target = load_cron_notify_target(&data_root).ok_or_else(|| {
        anyhow::anyhow!(
            "wechat cron_notify_target.json missing; send any message to the WeChat bot first"
        )
    })?;
    let api = Arc::new(WeChatApi::new(
        account.bot_token.clone(),
        account.base_url.clone(),
    ));
    let (target, target_refreshed) =
        resolve_outbound_target(&data_root, api.as_ref(), &target).await?;
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

async fn recover_and_retry_stale(prepared: &mut OutboundPrepared) -> bool {
    let Some(fresh) = try_recover_fresh_context(
        &prepared.data_root,
        Some(prepared.sender.api()),
        &prepared.target,
    )
    .await
    else {
        return false;
    };
    if fresh.context_token == prepared.target.context_token {
        return false;
    }
    prepared.target = fresh;
    prepared.target_refreshed = true;
    true
}

async fn send_text_with_context_retry(prepared: &mut OutboundPrepared, text: &str) -> Result<()> {
    let mut stale_retried = false;
    loop {
        match prepared
            .sender
            .send_text(
                &prepared.target.from_user_id,
                &prepared.target.context_token,
                text,
            )
            .await
        {
            Ok(()) => return Ok(()),
            Err(e) if is_stale_wechat_session(&e) && !stale_retried => {
                stale_retried = true;
                if recover_and_retry_stale(prepared).await {
                    continue;
                }
                let hint = stale_session_hint(&e);
                return Err(e).context(format!("wechat send failed{hint}"));
            }
            Err(e) => {
                let hint = stale_session_hint(&e);
                return Err(e).context(format!("wechat send failed{hint}"));
            }
        }
    }
}

async fn send_media_with_context_retry(
    prepared: &mut OutboundPrepared,
    path: &Path,
    caption: Option<&str>,
) -> Result<WeChatMediaDelivery> {
    let mut stale_retried = false;
    loop {
        match send_deliverable_path(
            &prepared.sender,
            &prepared.target.from_user_id,
            &prepared.target.context_token,
            path,
            caption,
        )
        .await
        {
            Ok(delivery) => return Ok(delivery),
            Err(e) if is_stale_wechat_session(&e) && !stale_retried => {
                stale_retried = true;
                if recover_and_retry_stale(prepared).await {
                    continue;
                }
                let hint = stale_session_hint(&e);
                return Err(e).context(format!("wechat media send failed{hint}"));
            }
            Err(e) => {
                let hint = stale_session_hint(&e);
                return Err(e).context(format!("wechat media send failed{hint}"));
            }
        }
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
    let mut prepared = prepare_outbound_sender(data_dir).await?;
    send_text_with_context_retry(&mut prepared, text).await?;
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

    let mut prepared = prepare_outbound_sender(data_dir).await?;
    let delivery = send_media_with_context_retry(&mut prepared, &path, caption).await?;

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
