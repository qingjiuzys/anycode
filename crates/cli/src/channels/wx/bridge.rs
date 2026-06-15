//! Polling loop, message state machine, and embedded `AgentRuntime`.

use super::approval::{wechat_approval_callback, ActiveChat, WechatApprovalGate};
use super::bridge_lock::BridgeLockGuard;
use super::cdn_media::{
    download_cdn_item_bytes, extract_user_text_and_image_item, first_plain_text_from_items,
    has_voice_item_without_stt, save_inbound_wx_media,
};
use super::commands::{route_command, CmdCtx, CmdOut};
use super::cron_notify::touch_outbound_context;
use super::deliverable::{
    assistant_history_with_paths, detect_resend_request, match_deliverable_for_resend,
    resolve_outbound_media_paths, send_deliverable_path, send_outbound_media_paths,
    with_deliverable_hint,
};
use super::fields::{
    i64_snake_camel, item_type, msgs_array, str_snake_camel, sync_buf_from_response,
};
use super::ilink::{load_sync_buf, save_sync_buf, WeChatApi, WxSender};
#[cfg(target_os = "macos")]
use super::image_ocr;
use super::permission::PermissionBroker;
use super::store::{
    add_chat_message, chat_history_text, deliverables_context_text, load_latest_account,
    load_session, load_wcc_config, record_session_deliverable, save_session, wcc_data_dir,
    AccountData, DeliverableSource, SessionState, WcSession, WccConfig,
};
use super::voice_stt;
use crate::app_config::{resolve_config_path, Config};
use crate::bootstrap::initialize_runtime;
use crate::i18n::{tr, tr_args};
use crate::tool_policy::ToolPolicyConfigSnapshot;
use anycode_agent::AgentRuntime;
use anycode_channels::profile_for_channel_type;
use anycode_core::prelude::*;
use anycode_core::strip_llm_reasoning_for_display;
use anycode_locale::{resolve_locale, AppLocale};
use anyhow::{Context, Result};
use base64::Engine;
use fluent_bundle::FluentArgs;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::sync::RwLock;

use super::config_watch::{
    reload_runtime_if_config_changed, spawn_config_file_watcher, ConfigReloadHandle,
};

const SESSION_EXPIRED: i64 = -14;
const MAX_MSG_IDS: usize = 1000;
const CHUNK_MAX: usize = 2048;

fn resolve_cwd(session: &WcSession, wcc: &WccConfig) -> PathBuf {
    let raw = if session.working_directory.is_empty() {
        wcc.working_directory.clone()
    } else {
        session.working_directory.clone()
    };
    let raw = if let Some(rest) = raw.strip_prefix("~/") {
        dirs::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| PathBuf::from(&raw))
    } else {
        PathBuf::from(&raw)
    };
    std::fs::canonicalize(&raw).unwrap_or(raw)
}

pub async fn run_wechat_daemon(
    app_config: &Config,
    config_file: Option<PathBuf>,
    ignore_approval: bool,
    data_dir: Option<PathBuf>,
    agent_type: String,
) -> Result<()> {
    let data_root = wcc_data_dir(data_dir);
    std::fs::create_dir_all(&data_root)?;

    let account = load_latest_account(&data_root)?;
    let wcc = load_wcc_config(&data_root);
    let session = load_session(&data_root, &account.account_id)?;
    let cwd = resolve_cwd(&session, &wcc);

    let api = Arc::new(WeChatApi::new(
        account.bot_token.clone(),
        account.base_url.clone(),
    ));
    let sender = Arc::new(
        WxSender::new(api.clone(), account.account_id.clone())
            .with_outbound_log(super::outbound_queue::wechat_outbound_log_path(&data_root)),
    );
    let broker = PermissionBroker::new(account.account_id.clone());
    let qbroker = Arc::new(super::super::wx_ask::WechatQuestionBroker::new());
    let ask_host =
        super::super::wx_ask::WechatAskUserQuestionHost::new(Arc::clone(&qbroker), sender.clone())
            .into_arc();

    let session_arc = Arc::new(Mutex::new(session));
    let wcc_arc = Arc::new(Mutex::new(wcc.clone()));
    let active_chat = Arc::new(Mutex::new(None::<ActiveChat>));
    let active_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>> = Arc::new(Mutex::new(None));
    let wx_turn_cancel: Arc<StdMutex<Option<Arc<AtomicBool>>>> = Arc::new(StdMutex::new(None));

    let gate = WechatApprovalGate::new(
        data_root.clone(),
        account.account_id.clone(),
        session_arc.clone(),
        active_chat.clone(),
        sender.clone(),
        broker.clone(),
    );

    // 微信桥的交互审批必须回到当前微信会话，不能退回本机 Workbench / CLI。
    let project_enabled = crate::workbench::project_skills::load_project_enabled_skills(&cwd).await;
    let runtime = Arc::new(RwLock::new(
        initialize_runtime(
            app_config,
            wechat_approval_callback(&gate, ignore_approval),
            Some(ask_host.clone()),
            crate::bootstrap::MemoryAttachMode::Exclusive,
            project_enabled,
        )
        .await
        .context("initialize_runtime")?,
    ));

    let last_config_mtime: Arc<StdMutex<Option<SystemTime>>> = Arc::new(StdMutex::new(
        resolve_config_path(config_file.clone())
            .ok()
            .and_then(|p| std::fs::metadata(&p).ok())
            .and_then(|m| m.modified().ok()),
    ));

    let reload = ConfigReloadHandle {
        runtime: Arc::clone(&runtime),
        config_file: config_file.clone(),
        ignore_approval,
        last_config_mtime: Arc::clone(&last_config_mtime),
        ask_user_question_host: Some(ask_host),
        approval_gate: Some(gate.clone()),
        tool_policy: Arc::new(StdMutex::new(ToolPolicyConfigSnapshot::from(app_config))),
    };
    if let Ok(p) = resolve_config_path(config_file.clone()) {
        spawn_config_file_watcher(reload.clone(), p);
    }

    let mut sa = FluentArgs::new();
    sa.set("id", account.account_id.clone());
    println!("{}", tr_args("wx-bridge-started", &sa));

    let broker_for_state = gate.permission_broker();
    let sched_runtime = Arc::clone(&runtime);
    let sched_sender = sender.clone();
    let st = BridgeState {
        data_root: data_root.clone(),
        account,
        wcc_arc,
        session_arc,
        active_chat,
        active_task,
        wx_turn_cancel,
        gate,
        broker: broker_for_state,
        qbroker,
        runtime,
        reload,
        sender,
        api,
        agent_type,
    };

    // 与 `CronCreate` / `anycode scheduler` 对齐：同机只应有一个调度循环；锁见 `scheduler::scheduler_lock_path`。
    let cwd_sched = {
        let session = st.session_arc.lock().await;
        let wcc = st.wcc_arc.lock().await;
        resolve_cwd(&session, &wcc)
    };
    let sched_cfg = app_config.clone();
    let delivery = crate::scheduler::CronDelivery::Wechat(crate::scheduler::SchedulerWechatHooks {
        data_root: data_root.clone(),
        sender: sched_sender,
    });
    crate::scheduler::spawn_embedded_scheduler(sched_cfg, cwd_sched, sched_runtime, delivery, 30);

    let _bridge_lock = BridgeLockGuard::acquire(&data_root).context("wechat bridge lock")?;

    run_monitor(st).await
}

#[derive(Clone)]
struct BridgeState {
    data_root: PathBuf,
    account: AccountData,
    wcc_arc: Arc<Mutex<WccConfig>>,
    session_arc: Arc<Mutex<WcSession>>,
    active_chat: Arc<Mutex<Option<ActiveChat>>>,
    active_task: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// 当前微信回合的协作式取消：`execute_task` 在工具/轮次边界检查；中断或 `/clear` 时置位。
    wx_turn_cancel: Arc<StdMutex<Option<Arc<AtomicBool>>>>,
    gate: WechatApprovalGate,
    broker: super::permission::PermissionBroker,
    qbroker: Arc<super::super::wx_ask::WechatQuestionBroker>,
    /// 与 `config.json` 同步；热更新时整实例替换，进行中任务仍持有旧 `Arc`。
    runtime: Arc<RwLock<Arc<AgentRuntime>>>,
    reload: ConfigReloadHandle,
    sender: Arc<WxSender>,
    api: Arc<WeChatApi>,
    agent_type: String,
}

async fn run_monitor(st: BridgeState) -> Result<()> {
    let mut recent_ids: HashSet<i64> = HashSet::new();
    let mut fail_streak: u32 = 0;

    loop {
        reload_runtime_if_config_changed(&st.reload).await;

        let buf = load_sync_buf(&st.data_root);
        let resp = match st
            .api
            .get_updates(if buf.is_empty() { None } else { Some(&buf) })
            .await
        {
            Ok(r) => r,
            Err(e) => {
                fail_streak += 1;
                tracing::error!(error = %e, fail_streak, "{}", tr("wx-log-getupdates-fail"));
                let ms = if fail_streak >= 3 {
                    30_000u64
                } else {
                    3_000u64
                };
                tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                continue;
            }
        };

        fail_streak = 0;

        if resp.get("ret").and_then(|x| x.as_i64()) == Some(SESSION_EXPIRED) {
            eprintln!("{}", tr("wx-session-expired"));
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            continue;
        }

        if let Some(b) = sync_buf_from_response(&resp) {
            let _ = save_sync_buf(&st.data_root, b);
        }

        if let Some(r) = resp.get("ret").and_then(|x| x.as_i64()) {
            if r != 0 && r != SESSION_EXPIRED {
                tracing::debug!(
                    ret = r,
                    retmsg = ?resp.get("retmsg").or_else(|| resp.get("retMsg")),
                    "{}",
                    tr("wx-log-getupdates-ret")
                );
            }
        }

        let msgs = msgs_array(&resp);

        if !msgs.is_empty() {
            tracing::info!(count = msgs.len(), "{}", tr("wx-log-msgs"));
        }

        for msg in msgs {
            // 与 openclaw-weixin `MessageType` 对齐：1=用户 2=机器人；不强制仅处理 1，避免错丢入站。
            let msg_type = i64_snake_camel(&msg, "message_type", "messageType").unwrap_or(0);
            if msg_type == 2 {
                tracing::debug!(msg_type, "wx-skip-bot-echo");
                continue;
            }
            let from = match str_snake_camel(&msg, "from_user_id", "fromUserId") {
                Some(f) => f.to_string(),
                None => {
                    tracing::debug!("{}", tr("wx-log-skip-no-from"));
                    continue;
                }
            };
            let items: Vec<_> = msg
                .get("item_list")
                .and_then(|x| x.as_array())
                .or_else(|| msg.get("itemList").and_then(|x| x.as_array()))
                .cloned()
                .unwrap_or_default();
            if items.is_empty() {
                tracing::debug!(?from, "{}", tr("wx-log-skip-empty-items"));
                continue;
            }
            let items_ref: Vec<_> = items.to_vec();
            let ctx_tok = str_snake_camel(&msg, "context_token", "contextToken")
                .unwrap_or("")
                .to_string();
            let mid = i64_snake_camel(&msg, "message_id", "messageId");

            if let Some(id) = mid {
                if recent_ids.contains(&id) {
                    continue;
                }
                recent_ids.insert(id);
                if recent_ids.len() > MAX_MSG_IDS {
                    let v: Vec<i64> = recent_ids.iter().copied().collect();
                    let drop = v.len() / 2;
                    for x in v.into_iter().take(drop) {
                        recent_ids.remove(&x);
                    }
                }
            }

            if !ctx_tok.trim().is_empty() {
                let _ = touch_outbound_context(&st.data_root, &from, &ctx_tok);
            }

            let st2 = BridgeState {
                data_root: st.data_root.clone(),
                account: AccountData {
                    bot_token: st.account.bot_token.clone(),
                    account_id: st.account.account_id.clone(),
                    base_url: st.account.base_url.clone(),
                    user_id: st.account.user_id.clone(),
                    created_at: st.account.created_at.clone(),
                },
                wcc_arc: st.wcc_arc.clone(),
                session_arc: st.session_arc.clone(),
                active_chat: st.active_chat.clone(),
                active_task: st.active_task.clone(),
                wx_turn_cancel: st.wx_turn_cancel.clone(),
                gate: st.gate.clone(),
                broker: st.broker.clone(),
                qbroker: st.qbroker.clone(),
                runtime: st.runtime.clone(),
                reload: st.reload.clone(),
                sender: st.sender.clone(),
                api: st.api.clone(),
                agent_type: st.agent_type.clone(),
            };

            tokio::spawn(async move {
                if let Err(e) = handle_message(st2, from, ctx_tok, items_ref).await {
                    tracing::error!(error = %e, "{}", tr("wx-log-handle-msg-fail"));
                }
            });
        }
    }
}

async fn handle_message(
    st: BridgeState,
    from_user_id: String,
    context_token: String,
    items: Vec<serde_json::Value>,
) -> Result<()> {
    let _ = touch_outbound_context(&st.data_root, &from_user_id, &context_token);
    let (body, media_item) = extract_user_text_and_image_item(&items);
    let cmd = first_plain_text_from_items(&items);

    if st.qbroker.try_resolve_numeric(&from_user_id, &body).await {
        return Ok(());
    }

    let mut session = st.session_arc.lock().await;
    let wcc = st.wcc_arc.lock().await.clone();

    if !wcc.working_directory.is_empty()
        && session.working_directory
            == std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
    {
        session.working_directory = wcc.working_directory.clone();
        let _ = save_session(&st.data_root, &st.account.account_id, &session);
    }

    if session.state == SessionState::Processing {
        if cmd.trim_start().starts_with("/clear") {
            if let Some(f) = st.wx_turn_cancel.lock().unwrap().as_ref() {
                f.store(true, Ordering::SeqCst);
            }
            if let Some(h) = st.active_task.lock().await.take() {
                h.abort();
            }
            session.state = SessionState::Idle;
            let _ = save_session(&st.data_root, &st.account.account_id, &session);
        } else if !cmd.trim_start().starts_with('/') {
            if let Some(f) = st.wx_turn_cancel.lock().unwrap().as_ref() {
                f.store(true, Ordering::SeqCst);
            }
            if let Some(h) = st.active_task.lock().await.take() {
                h.abort();
            }
            session.state = SessionState::Idle;
            add_chat_message(
                &mut session,
                "assistant",
                &tr("wx-history-turn-interrupted"),
            );
            let _ = save_session(&st.data_root, &st.account.account_id, &session);
            drop(session);
            let _ = st
                .sender
                .send_text(&from_user_id, &context_token, &tr("wx-interrupt-new-msg"))
                .await;
            // Re-lock session for the rest of `handle_message`.
            session = st.session_arc.lock().await;
        } else if !cmd.starts_with("/status") && !cmd.starts_with("/help") {
            drop(session);
            return Ok(());
        }
    }

    if session.state == SessionState::Idle && st.broker.is_timed_out().await {
        let lower = body.to_lowercase();
        if matches!(lower.as_str(), "y" | "yes" | "n" | "no") {
            st.broker.clear_timed_out().await;
            st.sender
                .send_text(&from_user_id, &context_token, &tr("wx-perm-timeout"))
                .await?;
        }
        drop(session);
        return Ok(());
    }

    if session.state == SessionState::WaitingPermission {
        if st.broker.get_pending().await.is_none() {
            session.state = SessionState::Idle;
            let _ = save_session(&st.data_root, &st.account.account_id, &session);
            st.sender
                .send_text(&from_user_id, &context_token, &tr("wx-perm-stale"))
                .await?;
            drop(session);
            return Ok(());
        }
        let lower = body.to_lowercase();
        let reply = if matches!(lower.as_str(), "y" | "yes") {
            let ok = st.broker.resolve(true).await;
            if ok {
                tr("wx-perm-allowed")
            } else {
                tr("wx-perm-fail")
            }
        } else if matches!(lower.as_str(), "n" | "no") {
            let ok = st.broker.resolve(false).await;
            if ok {
                tr("wx-perm-denied")
            } else {
                tr("wx-perm-fail")
            }
        } else {
            tr("wx-perm-wait")
        };
        st.sender
            .send_text(&from_user_id, &context_token, &reply)
            .await?;
        drop(session);
        return Ok(());
    }

    if cmd.starts_with('/') {
        if cmd.trim_start().starts_with("/clear") {
            let _ = st.broker.reject_pending().await;
        }
        let mut wcc_mut = st.wcc_arc.lock().await.clone();
        let mut ctx = CmdCtx {
            data_root: &st.data_root,
            account_id: &st.account.account_id,
            session: &mut session,
            wcc: &mut wcc_mut,
        };
        match route_command(&cmd, &mut ctx)? {
            CmdOut::Reply(s) => {
                let _ = save_session(&st.data_root, &st.account.account_id, &session);
                *st.wcc_arc.lock().await = wcc_mut.clone();
                drop(session);
                st.sender
                    .send_text(&from_user_id, &context_token, &s)
                    .await?;
                return Ok(());
            }
            CmdOut::Nothing => {
                *st.wcc_arc.lock().await = wcc_mut;
            }
        }
    }

    if body.is_empty() && media_item.is_none() {
        drop(session);
        if has_voice_item_without_stt(&items) {
            if let Some(text) = voice_stt::transcribe_voice_items(&items).await {
                run_agent_pipeline(st, from_user_id, context_token, text.clone(), text, None)
                    .await?;
                return Ok(());
            }
        }
        let reply = if has_voice_item_without_stt(&items) {
            tr("wx-voice-no-stt")
        } else {
            tr("wx-unsupported-msg")
        };
        st.sender
            .send_text(&from_user_id, &context_token, &reply)
            .await?;
        return Ok(());
    }

    let user_line = if !body.is_empty() {
        body.clone()
    } else if let Some(m) = &media_item {
        match item_type(m) {
            2 => tr("wx-analyze-image"),
            _ => tr("wx-analyze-attachment"),
        }
    } else {
        String::new()
    };

    if !body.is_empty() && media_item.is_none() {
        if let Some(intent) = detect_resend_request(&body) {
            if let Some(path) = match_deliverable_for_resend(&session.deliverables, intent) {
                add_chat_message(&mut session, "user", &body);
                let _ = save_session(&st.data_root, &st.account.account_id, &session);
                drop(session);
                return handle_resend_deliverable(st, from_user_id, context_token, path).await;
            }
        }
    }

    drop(session);
    run_agent_pipeline(
        st,
        from_user_id,
        context_token,
        user_line,
        body,
        media_item.cloned(),
    )
    .await
}

async fn run_agent_pipeline(
    st: BridgeState,
    from_user_id: String,
    context_token: String,
    user_line: String,
    body: String,
    media_item: Option<serde_json::Value>,
) -> Result<()> {
    let chat = ActiveChat {
        from_user_id: from_user_id.clone(),
        context_token: context_token.clone(),
    };
    st.gate.set_active_chat(Some(chat)).await;

    {
        let mut session = st.session_arc.lock().await;
        session.state = SessionState::Processing;
        let analyze = tr("wx-analyze-image");
        let att = tr("wx-analyze-attachment");
        let content = if !body.is_empty() {
            body.clone()
        } else if let Some(m) = &media_item {
            if item_type(m) == 2 && user_line == analyze {
                tr("wx-chat-image-marker")
            } else if user_line == att || user_line == analyze {
                tr("wx-chat-attachment-marker")
            } else {
                user_line.clone()
            }
        } else {
            user_line.clone()
        };
        add_chat_message(&mut session, "user", &content);
        let _ = save_session(&st.data_root, &st.account.account_id, &session);
    }

    let _ = st
        .sender
        .send_text(&from_user_id, &context_token, &tr("wx-task-received"))
        .await;

    let mut user_vision_images: Vec<anycode_core::VisionImage> = Vec::new();
    let mut inbound_saved: Option<(PathBuf, Option<String>)> = None;
    let media_note = if let Some(ref it) = media_item {
        let t = item_type(it);
        let kind = match t {
            2 => tr("wx-media-kind-image"),
            3 => tr("wx-media-kind-voice"),
            4 => tr("wx-media-kind-file"),
            5 => tr("wx-media-kind-video"),
            _ => "media".to_string(),
        };
        match download_cdn_item_bytes(st.api.http_client(), it).await {
            Some((mime, bytes)) => {
                if t == 2 {
                    #[cfg(target_os = "macos")]
                    if let Some(text) = image_ocr::ocr_inbound_image(&mime, &bytes) {
                        let mut oa = FluentArgs::new();
                        oa.set("text", text);
                        Some(tr_args("wx-llm-image-ocr-note", &oa))
                    } else {
                        user_vision_images.push(anycode_core::VisionImage::new(
                            mime.clone(),
                            base64::engine::general_purpose::STANDARD.encode(&bytes),
                        ));
                        if let Ok(path) = save_inbound_wx_media(&st.data_root, it, &mime, &bytes) {
                            inbound_saved = Some((path.clone(), Some(mime.clone())));
                            let mut sa = FluentArgs::new();
                            sa.set("path", path.display().to_string());
                            Some(tr_args("wx-llm-image-saved-note", &sa))
                        } else {
                            None
                        }
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        user_vision_images.push(anycode_core::VisionImage::new(
                            mime.clone(),
                            base64::engine::general_purpose::STANDARD.encode(&bytes),
                        ));
                        None
                    }
                } else if let Ok(path) = save_inbound_wx_media(&st.data_root, it, &mime, &bytes) {
                    inbound_saved = Some((path.clone(), Some(mime.clone())));
                    let mut ia = FluentArgs::new();
                    ia.set("mime", mime);
                    ia.set("path", path.display().to_string());
                    ia.set("kind", kind);
                    Some(tr_args("wx-llm-attachment-note", &ia))
                } else {
                    let mut fa = FluentArgs::new();
                    fa.set("kind", kind);
                    Some(tr_args("wx-llm-attachment-fail", &fa))
                }
            }
            None => {
                if t == 2 {
                    Some(tr("wx-llm-image-fail"))
                } else {
                    let mut fa = FluentArgs::new();
                    fa.set("kind", kind);
                    Some(tr_args("wx-llm-attachment-fail", &fa))
                }
            }
        }
    } else {
        None
    };

    if let Some((path, mime)) = inbound_saved {
        let mut session = st.session_arc.lock().await;
        record_session_deliverable(
            &mut session,
            &path,
            DeliverableSource::Inbound,
            false,
            mime,
            None,
        );
        if let Some(last) = session.chat_history.last_mut() {
            if last.role == "user" {
                let mut pa = FluentArgs::new();
                pa.set("path", path.display().to_string());
                last.content
                    .push_str(&tr_args("wx-chat-attachment-path-suffix", &pa));
            }
        }
        let _ = save_session(&st.data_root, &st.account.account_id, &session);
    }

    run_agent_pipeline_with_media_note(
        st,
        from_user_id,
        context_token,
        user_line,
        body,
        media_item,
        user_vision_images,
        media_note,
    )
    .await
}

async fn run_agent_pipeline_with_media_note(
    st: BridgeState,
    from_user_id: String,
    context_token: String,
    user_line: String,
    body: String,
    _media_item: Option<serde_json::Value>,
    user_vision_images: Vec<anycode_core::VisionImage>,
    media_note: Option<String>,
) -> Result<()> {
    let wcc = st.wcc_arc.lock().await.clone();
    let channel_profile = profile_for_channel_type(&ChannelType::WeChat);
    let prompt_body = {
        let session = st.session_arc.lock().await;
        let mut hist = session.clone();
        if !hist.chat_history.is_empty() {
            hist.chat_history.pop();
        }
        let hist_txt = chat_history_text(&hist, Some(40));
        let mut ha = FluentArgs::new();
        ha.set("hist", hist_txt);
        ha.set(
            "user",
            if !body.is_empty() {
                body.clone()
            } else {
                user_line.clone()
            },
        );
        let mut p = tr_args("wx-llm-history-wrap", &ha);
        let deliv_txt = deliverables_context_text(&session, Some(10));
        if !deliv_txt.is_empty() {
            let mut da = FluentArgs::new();
            da.set("files", deliv_txt);
            p.push_str(&tr_args("wx-llm-deliverables-wrap", &da));
        }
        if let Some(note) = media_note {
            p.push_str(&note);
        }
        p
    };

    let wx_system_append = wcc.system_prompt.clone().filter(|s| !s.trim().is_empty());

    let (cwd, runtime_mode) = {
        let session = st.session_arc.lock().await;
        (
            resolve_cwd(&session, &wcc),
            session
                .runtime_mode
                .clone()
                .or_else(|| wcc.runtime_mode.clone())
                .unwrap_or_else(|| channel_profile.default_mode.as_str().to_string()),
        )
    };

    let rt = st.runtime.read().await.clone();
    let (tool_deny_names, tool_deny_prefixes) = {
        let snap = st.reload.tool_policy.lock().expect("tool_policy lock");
        crate::tool_policy::channel_tool_filters_from_snapshot(&snap)
    };
    let agent = resolve_channel_agent(
        &runtime_mode,
        &st.agent_type,
        channel_profile.assistant_agent,
    );
    let data_root = st.data_root.clone();
    let account_id = st.account.account_id.clone();
    let sender = st.sender.clone();
    let session_arc = st.session_arc.clone();
    let gate = st.gate.clone();
    let active_task = st.active_task.clone();
    let wx_turn_cancel_slot = st.wx_turn_cancel.clone();

    let h = tokio::spawn(async move {
        let coop = Arc::new(AtomicBool::new(false));
        *wx_turn_cancel_slot.lock().unwrap() = Some(coop.clone());
        let cwd_for_media = cwd.clone();

        let _ = sender
            .send_text(&from_user_id, &context_token, &tr("wx-task-running"))
            .await;

        let task =
            crate::task_builders::build_wechat_task(crate::task_builders::WechatTaskParams {
                agent,
                prompt: prompt_body,
                working_directory: cwd,
                system_prompt_append: Some(build_wechat_system_append(
                    wx_system_append.as_deref(),
                    &runtime_mode,
                    channel_profile.id,
                    channel_profile.assistant_agent,
                )),
                tool_deny_names,
                tool_deny_prefixes,
                nested_cancel: Some(coop),
                user_vision_images,
            });

        let result = super::super::wx_ask::with_wechat_task_scope(
            from_user_id.clone(),
            context_token.clone(),
            rt.execute_task(task),
        )
        .await;
        *wx_turn_cancel_slot.lock().unwrap() = None;
        rt.sync_memory_durability();
        gate.set_active_chat(None).await;

        let mut session = session_arc.lock().await;
        let mut outbound_media_paths = Vec::new();
        let reply = match result {
            Ok(TaskResult::Success { output, artifacts }) => {
                let cleaned = sanitize_wechat_reply_output(&output);
                outbound_media_paths =
                    resolve_outbound_media_paths(&artifacts, &output, Some(&cwd_for_media)).await;
                for path in &outbound_media_paths {
                    record_session_deliverable(
                        &mut session,
                        path,
                        DeliverableSource::Outbound,
                        true,
                        None,
                        None,
                    );
                }
                let history_reply = assistant_history_with_paths(&cleaned, &outbound_media_paths);
                add_chat_message(&mut session, "assistant", &history_reply);
                with_deliverable_hint(cleaned, &output)
            }
            Ok(TaskResult::Failure { error, details }) => {
                let mut ea = FluentArgs::new();
                ea.set("err", error.to_string());
                let mut reply = tr_args("wx-task-fail", &ea);
                let history_msg = tr_args("wx-history-turn-failed", &ea);
                add_chat_message(&mut session, "assistant", &history_msg);
                if let Some(ex) =
                    crate::channel_task::im_task_failure_detail_excerpt(details.as_deref(), 900)
                {
                    let mut da = FluentArgs::new();
                    da.set("details", ex);
                    reply.push_str(&tr_args("wx-task-fail-details", &da));
                }
                reply
            }
            Ok(TaskResult::Partial { success, remaining }) => {
                let combined = format!("{success}\n{remaining}");
                let t = sanitize_wechat_reply_output(&combined);
                outbound_media_paths =
                    resolve_outbound_media_paths(&[], &combined, Some(&cwd_for_media)).await;
                for path in &outbound_media_paths {
                    record_session_deliverable(
                        &mut session,
                        path,
                        DeliverableSource::Outbound,
                        true,
                        None,
                        None,
                    );
                }
                let history_reply = assistant_history_with_paths(&t, &outbound_media_paths);
                add_chat_message(&mut session, "assistant", &history_reply);
                t
            }
            Err(e) => {
                let mut ea = FluentArgs::new();
                ea.set("err", e.to_string());
                let history_msg = tr_args("wx-history-turn-failed", &ea);
                add_chat_message(&mut session, "assistant", &history_msg);
                tr_args("wx-exec-error", &ea)
            }
        };
        session.state = SessionState::Idle;
        let _ = save_session(&data_root, &account_id, &session);
        drop(session);

        for chunk in split_message(&reply, CHUNK_MAX) {
            if let Err(e) = sender
                .send_text(&from_user_id, &context_token, &chunk)
                .await
            {
                tracing::error!(
                    error = %e,
                    chunk_len = chunk.len(),
                    "wx reply chunk send failed after retries"
                );
            }
        }
        if !outbound_media_paths.is_empty() {
            if let Err(e) = send_outbound_media_paths(
                &sender,
                &from_user_id,
                &context_token,
                &outbound_media_paths,
            )
            .await
            {
                tracing::warn!(error = %e, "wx outbound media batch send failed");
            }
        }
        let _ = active_task.lock().await.take();
    });

    *st.active_task.lock().await = Some(h);
    Ok(())
}

async fn handle_resend_deliverable(
    st: BridgeState,
    from_user_id: String,
    context_token: String,
    path: PathBuf,
) -> Result<()> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("file")
        .to_string();
    let _ = st
        .sender
        .send_text(&from_user_id, &context_token, &tr("wx-task-received"))
        .await;

    let send_result =
        send_deliverable_path(&st.sender, &from_user_id, &context_token, &path, None).await;

    let reply = match send_result {
        Ok(_) => {
            let mut a = FluentArgs::new();
            a.set("name", name.clone());
            tr_args("wx-resend-ok", &a)
        }
        Err(e) => {
            let mut a = FluentArgs::new();
            a.set("err", e.to_string());
            tr_args("wx-resend-fail", &a)
        }
    };

    {
        let mut session = st.session_arc.lock().await;
        add_chat_message(&mut session, "assistant", &reply);
        record_session_deliverable(
            &mut session,
            &path,
            DeliverableSource::Outbound,
            true,
            None,
            Some("resend".into()),
        );
        let _ = save_session(&st.data_root, &st.account.account_id, &session);
    }

    for chunk in split_message(&reply, CHUNK_MAX) {
        st.sender
            .send_text(&from_user_id, &context_token, &chunk)
            .await?;
    }
    Ok(())
}

fn resolve_channel_agent(
    runtime_mode: &str,
    bridge_agent: &str,
    channel_default_agent: &str,
) -> String {
    match RuntimeMode::parse(runtime_mode) {
        Some(RuntimeMode::Plan) => "plan".to_string(),
        Some(RuntimeMode::Explore) => "explore".to_string(),
        Some(RuntimeMode::Goal) => "goal".to_string(),
        Some(RuntimeMode::Code) => "general-purpose".to_string(),
        Some(RuntimeMode::Channel) => channel_default_agent.to_string(),
        Some(RuntimeMode::General) | None => {
            if bridge_agent.trim().is_empty() {
                channel_default_agent.to_string()
            } else {
                bridge_agent.to_string()
            }
        }
    }
}

fn build_wechat_system_append(
    existing: Option<&str>,
    runtime_mode: &str,
    channel_id: &str,
    channel_default_agent: &str,
) -> String {
    let locale = resolve_locale();
    let mut sections = vec![format!(
        "## Channel Runtime\nchannel={channel_id}\nruntime_mode={runtime_mode}\ndefault_channel_agent={channel_default_agent}\nFor WeChat channel mode, default to workspace-assistant behavior. Only perform direct coding when the user explicitly asks to modify code."
    )];
    sections.push(wechat_output_contract(locale));
    sections.push(wechat_reply_language(locale));
    sections.push(crate::channel_task::im_channel_cron_scheduling_hint());
    sections.push(wechat_outbound_files_hint(locale));
    if let Some(existing) = existing {
        if !existing.trim().is_empty() {
            sections.push(existing.trim().to_string());
        }
    }
    sections.join("\n\n")
}

fn wechat_reply_language(locale: AppLocale) -> String {
    let directive = match locale {
        AppLocale::ZhHans => {
            "除非用户明确用其它语言提问，否则**全文仅中文**（代码、命令、路径、标识符除外）。\
             禁止在中文答案后再写英文分析、总结或复述（如 \"Let me analyze…\"、\"First query…\"）。\
             不要中英双语重复同一内容。"
        }
        AppLocale::En => {
            "Reply entirely in English unless the user writes in another language. \
             Do not append a second-language recap or analysis after the main answer."
        }
    };
    format!("## Reply language\n\n{directive}")
}

fn wechat_output_contract(locale: AppLocale) -> String {
    match locale {
        AppLocale::ZhHans => "## WeChat 输出契约（必须遵守）\n\
             - 用户只看到你这一条最终回复。禁止输出：思考过程、自我解说（如「太好了」「我需要…」「从页面中可以提取」「根据系统提示我应该…」）、工具调用是否成功的说明、同一段数据的重复粘贴。\n\
             - 工具/Web 结果只内化进答案：用一小段话或一层列表给出结论即可；不要先写长段再抄一遍字段。\n\
             - 天气/事实类：几句带关键数字即可，不要「摘要 + 再列一遍同样指标」。\n\
             - 禁止输出 <thought>、<thinking> 等标签或任何「推理草稿」块；只输出给用户的一句话/一小段结论。"
            .to_string(),
        AppLocale::En => "## WeChat output contract (required)\n\
             - The user sees only your final reply. Do not output reasoning, self-narration (e.g. \"Great!\", \"I need to…\", \"From the page I can extract…\"), tool success notes, or duplicate copies of the same facts.\n\
             - Internalize tool/web results into a short answer or one list level; do not paste a long dump then repeat the same fields.\n\
             - Weather/facts: a few lines with key numbers are enough—no \"summary + same metrics again\".\n\
             - Do not output <thought>, <thinking>, or any draft reasoning block—only the user-facing conclusion."
            .to_string(),
    }
}

fn wechat_outbound_files_hint(locale: AppLocale) -> String {
    match locale {
        AppLocale::ZhHans => "## WeChat 出站文件\n\
             - 需要发给用户的文件（报告、图片、导出包），请在最终回复写出**绝对路径**；桥接会自动通过 CDN 发送。\n\
             - 截图优先 `.png`/`.jpg`；文档 `.pdf`/`.docx`/`.zip`；短视频 `.mp4`。\n\
             - 单文件 ≤10MB；更大文件只告知本地路径。"
            .to_string(),
        AppLocale::En => "## WeChat outbound files\n\
             - When you create files the user should receive (reports, images, exports), include the **absolute path** in your final reply; the bridge auto-sends via WeChat CDN.\n\
             - Prefer `.png`/`.jpg` for screenshots; `.pdf`/`.docx`/`.zip` for documents; `.mp4` for short videos.\n\
             - Single file limit: 10 MB; larger files — tell the user the local path only."
            .to_string(),
    }
}

fn contains_cjk(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(
            c,
            '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{F900}'..='\u{FAFF}'
        )
    })
}

fn is_latin_meta_line(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() || contains_cjk(t) {
        return false;
    }
    const PREFIXES: &[&str] = &[
        "Let me analyze",
        "Let me combine",
        "Let me check",
        "Let me ",
        "First query",
        "Second query",
        "Third query",
        "Now I have",
        "Now I can",
        "Since ",
        "Both show",
        "The first shows",
        "The second",
        "Great!",
        "I need to",
        "From the page",
        "According to the system",
        "According to",
        "WebFetch",
        "Wait,",
    ];
    PREFIXES.iter().any(|p| t.starts_with(p))
}

fn is_cjk_meta_line(line: &str) -> bool {
    let t = line.trim();
    if t.is_empty() || !contains_cjk(t) {
        return false;
    }
    const PREFIXES: &[&str] = &[
        "太好了",
        "从页面",
        "我需要",
        "根据系统",
        "让我先",
        "让我来",
        "成功获取",
    ];
    PREFIXES.iter().any(|p| t.starts_with(p))
}

/// After a locale-appropriate answer, models sometimes append foreign-language analysis—drop it.
fn truncate_trailing_foreign_meta(text: &str, locale: AppLocale) -> String {
    match locale {
        AppLocale::ZhHans => {
            if !contains_cjk(text) {
                return text.to_string();
            }
            for marker in [
                "Let me analyze",
                "Let me combine",
                "First query",
                "Now I have",
                "Now I can",
            ] {
                if let Some(pos) = text.find(marker) {
                    return text[..pos].trim().to_string();
                }
            }
            let lines: Vec<&str> = text.lines().collect();
            let mut seen_cjk = false;
            for (i, line) in lines.iter().enumerate() {
                if contains_cjk(line) && line.trim().len() > 1 {
                    seen_cjk = true;
                } else if seen_cjk && is_latin_meta_line(line) {
                    return lines[..i].join("\n").trim().to_string();
                }
            }
            text.to_string()
        }
        AppLocale::En => {
            if contains_cjk(text) && !text.chars().any(|c| c.is_ascii_alphabetic()) {
                return text.to_string();
            }
            let lines: Vec<&str> = text.lines().collect();
            let mut seen_latin = false;
            for (i, line) in lines.iter().enumerate() {
                if line.chars().any(|c| c.is_ascii_alphabetic()) && !is_cjk_meta_line(line) {
                    seen_latin = true;
                } else if seen_latin && is_cjk_meta_line(line) {
                    return lines[..i].join("\n").trim().to_string();
                }
            }
            text.to_string()
        }
    }
}

/// 去掉常见「废话」行（模型仍可能漏网，主要靠 system 约束）。
fn sanitize_wechat_reply_output(text: &str) -> String {
    let locale = resolve_locale();
    let text = strip_llm_reasoning_for_display(text);
    const DROP_LINE_PREFIXES: &[&str] = &[
        "太好了",
        "从页面内容",
        "从页面中",
        "从页面",
        "我需要向用户",
        "我需要",
        "根据系统提示",
        "根据系统",
        "我可以向用户",
        "我可以简洁",
        "让我来向",
        "让我向用户",
        "让我先",
        "成功获取了",
        "成功获取",
        "Great!",
        "I need to",
        "From the page",
        "According to the system",
        "WebFetch",
        "Let me analyze",
        "Let me combine",
        "Let me check",
        "First query",
        "Second query",
        "Now I have",
        "Now I can",
    ];
    let mut kept: Vec<String> = Vec::new();
    for line in text.lines() {
        let t = line.trim_start();
        if t.is_empty() {
            kept.push(String::new());
            continue;
        }
        let drop = DROP_LINE_PREFIXES.iter().any(|p| t.starts_with(p));
        if !drop {
            kept.push(line.to_string());
        }
    }
    while kept.last().map(|s| s.trim().is_empty()).unwrap_or(false) {
        kept.pop();
    }
    let mut s = truncate_trailing_foreign_meta(&kept.join("\n"), locale);
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    s = s.trim().to_string();
    while s.contains("。。") {
        s = s.replace("。。", "。");
    }
    s
}

fn split_message(text: &str, max_chars: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![];
    }
    if text.chars().count() <= max_chars {
        return vec![text.to_string()];
    }
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in text.chars() {
        buf.push(ch);
        if buf.chars().count() >= max_chars {
            out.push(buf.trim_end().to_string());
            buf.clear();
        }
    }
    if !buf.trim().is_empty() {
        out.push(buf.trim_end().to_string());
    }
    out
}

#[cfg(test)]
mod wechat_sanitize_tests {
    use super::sanitize_wechat_reply_output;

    #[test]
    fn drops_meta_lines_keeps_weather_body() {
        let raw =
            "## 杭州\n阴 29℃\n\n太好了！WebFetch 成功获取了天气。\n\n从页面中可以提取到：\n- 温度";
        let out = sanitize_wechat_reply_output(raw);
        assert!(out.contains("杭州"));
        assert!(!out.contains("太好了"));
        assert!(!out.contains("从页面"));
    }

    #[test]
    fn strips_thought_block_before_answer() {
        let raw = r#"<thought>The output is "hangzhou: ⛅ +72°F".
Wait, let me check.</thought>杭州当前天气：多云，约 22°C。"#;
        let out = sanitize_wechat_reply_output(raw);
        assert!(!out.to_lowercase().contains("<thought"));
        assert!(out.contains("杭州"));
        assert!(out.contains("22"));
    }

    #[test]
    fn strips_thinking_tag_variant() {
        let raw = "<thinking>draft</thinking>\n\n答案：1";
        let out = sanitize_wechat_reply_output(raw);
        assert_eq!(out, "答案：1");
    }

    #[test]
    fn strips_trailing_english_analysis_after_chinese_weather() {
        let raw = "杭州青山湖这边当前 26°C，多云。\n建议带伞。\n\
            Let me analyze the results:\n\
            First query for \"Qingshan Lake Hangzhou\": Partly Cloudy\n\
            Since Qingshan Lake is in Lin'an district, the second result is more location-specific.";
        let out = sanitize_wechat_reply_output(raw);
        assert!(out.contains("杭州青山湖"));
        assert!(out.contains("建议带伞"));
        assert!(!out.to_lowercase().contains("let me analyze"));
        assert!(!out.contains("First query"));
    }
}
