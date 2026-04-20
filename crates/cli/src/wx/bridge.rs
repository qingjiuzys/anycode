//! Polling loop, message state machine, and embedded `AgentRuntime`.

use crate::app_config::{resolve_config_path, Config};
use crate::bootstrap::initialize_runtime;
use crate::i18n::{tr, tr_args};
use crate::wx::approval::{ActiveChat, WechatApprovalGate};
use crate::wx::cdn_media::{download_image_from_item, extract_user_text_and_image_item};
use crate::wx::commands::{route_command, CmdCtx, CmdOut};
use crate::wx::fields::{i64_snake_camel, msgs_array, str_snake_camel, sync_buf_from_response};
use crate::wx::ilink::{load_sync_buf, save_sync_buf, WeChatApi, WxSender};
use crate::wx::permission::PermissionBroker;
use crate::wx::store::{
    add_chat_message, chat_history_text, load_latest_account, load_session, load_wcc_config,
    save_session, wcc_data_dir, AccountData, SessionState, WcSession, WccConfig,
};
use anycode_agent::AgentRuntime;
use anycode_channels::profile_for_channel_type;
use anycode_core::prelude::*;
use anycode_core::strip_llm_reasoning_for_display;
use anyhow::{Context, Result};
use fluent_bundle::FluentArgs;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::config_watch::{
    reload_runtime_if_config_changed, spawn_config_file_watcher, ConfigReloadHandle,
};

const SESSION_EXPIRED: i64 = -14;
const MAX_MSG_IDS: usize = 1000;
const CHUNK_MAX: usize = 2048;

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

    let api = Arc::new(WeChatApi::new(
        account.bot_token.clone(),
        account.base_url.clone(),
    ));
    let sender = Arc::new(WxSender::new(api.clone(), account.account_id.clone()));
    let broker = PermissionBroker::new(account.account_id.clone());

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

    // 通道模式与 Telegram/Discord 一致：工具走自动策略（无终端交互审批）。
    // `WechatApprovalGate` 仍用于会话路由与其它微信侧逻辑。
    let runtime = Arc::new(RwLock::new(
        initialize_runtime(app_config, None, None)
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
    };
    if let Ok(p) = resolve_config_path(config_file.clone()) {
        spawn_config_file_watcher(reload.clone(), p);
    }

    let mut sa = FluentArgs::new();
    sa.set("id", account.account_id.clone());
    println!("{}", tr_args("wx-bridge-started", &sa));

    let broker_for_state = gate.permission_broker();
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
        runtime,
        reload,
        sender,
        api,
        agent_type,
    };

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
    broker: crate::wx::permission::PermissionBroker,
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
            let msg_type = i64_snake_camel(&msg, "message_type", "messageType").unwrap_or(0);
            if msg_type != 1 {
                tracing::debug!(msg_type, "{}", tr("wx-log-skip-non-user"));
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
    let (user_text, image_item) = extract_user_text_and_image_item(&items);

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
        if user_text.trim_start().starts_with("/clear") {
            if let Some(f) = st.wx_turn_cancel.lock().unwrap().as_ref() {
                f.store(true, Ordering::SeqCst);
            }
            if let Some(h) = st.active_task.lock().await.take() {
                h.abort();
            }
            session.state = SessionState::Idle;
            let _ = save_session(&st.data_root, &st.account.account_id, &session);
        } else if !user_text.trim_start().starts_with('/') {
            if let Some(f) = st.wx_turn_cancel.lock().unwrap().as_ref() {
                f.store(true, Ordering::SeqCst);
            }
            if let Some(h) = st.active_task.lock().await.take() {
                h.abort();
            }
            session.state = SessionState::Idle;
            let _ = save_session(&st.data_root, &st.account.account_id, &session);
            drop(session);
            let _ = st
                .sender
                .send_text(&from_user_id, &context_token, &tr("wx-interrupt-new-msg"))
                .await;
            // Re-lock session for the rest of `handle_message`.
            session = st.session_arc.lock().await;
        } else if !user_text.starts_with("/status") && !user_text.starts_with("/help") {
            drop(session);
            return Ok(());
        }
    }

    if session.state == SessionState::Idle && st.broker.is_timed_out().await {
        let lower = user_text.to_lowercase();
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
        let lower = user_text.to_lowercase();
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

    if user_text.starts_with('/') {
        if user_text.trim_start().starts_with("/clear") {
            let _ = st.broker.reject_pending().await;
        }
        let mut wcc_mut = st.wcc_arc.lock().await.clone();
        let mut ctx = CmdCtx {
            data_root: &st.data_root,
            account_id: &st.account.account_id,
            session: &mut session,
            wcc: &mut wcc_mut,
        };
        match route_command(&user_text, &mut ctx)? {
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

    if user_text.is_empty() && image_item.is_none() {
        drop(session);
        st.sender
            .send_text(&from_user_id, &context_token, &tr("wx-unsupported-msg"))
            .await?;
        return Ok(());
    }

    let user_line = if user_text.is_empty() {
        tr("wx-analyze-image")
    } else {
        user_text.clone()
    };
    drop(session);
    run_agent_pipeline(
        st,
        from_user_id,
        context_token,
        user_line,
        image_item.cloned(),
    )
    .await
}

async fn run_agent_pipeline(
    st: BridgeState,
    from_user_id: String,
    context_token: String,
    user_line: String,
    image_item: Option<serde_json::Value>,
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
        let content = if user_line == analyze && image_item.is_some() {
            tr("wx-chat-image-marker")
        } else {
            user_line.clone()
        };
        add_chat_message(&mut session, "user", &content);
        let _ = save_session(&st.data_root, &st.account.account_id, &session);
    }

    let image_note = if let Some(ref it) = image_item {
        match download_image_from_item(st.api.http_client(), it).await {
            Some((mime, b64)) => {
                let mut ia = FluentArgs::new();
                ia.set("mime", mime);
                ia.set("len", b64.len() as i64);
                Some(tr_args("wx-llm-image-note", &ia))
            }
            None => Some(tr("wx-llm-image-fail")),
        }
    } else {
        None
    };

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
        ha.set("user", user_line.clone());
        let mut p = tr_args("wx-llm-history-wrap", &ha);
        if let Some(note) = image_note {
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

    let (progress_tx, mut progress_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let progress_user = from_user_id.clone();
    let progress_ctx = context_token.clone();
    let progress_sender = sender.clone();
    let progress_worker = tokio::spawn(async move {
        while let Some(line) = progress_rx.recv().await {
            let line = strip_llm_reasoning_for_display(&line);
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            let _ = progress_sender
                .send_text(&progress_user, &progress_ctx, t)
                .await;
        }
    });

    let h = tokio::spawn(async move {
        let coop = Arc::new(AtomicBool::new(false));
        *wx_turn_cancel_slot.lock().unwrap() = Some(coop.clone());

        let task = Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new(agent),
            prompt: prompt_body,
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: cwd.to_string_lossy().to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: Some(build_wechat_system_append(
                    wx_system_append.as_deref(),
                    &runtime_mode,
                    channel_profile.id,
                    channel_profile.assistant_agent,
                )),
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: Some(coop),
                channel_progress_tx: Some(progress_tx),
            },
            created_at: chrono::Utc::now(),
        };

        let result = rt.execute_task(task).await;
        *wx_turn_cancel_slot.lock().unwrap() = None;
        let _ = progress_worker.await;
        rt.sync_memory_durability();
        gate.set_active_chat(None).await;

        let mut session = session_arc.lock().await;
        let reply = match result {
            Ok(TaskResult::Success { output, .. }) => {
                let cleaned = sanitize_wechat_reply_output(&output);
                add_chat_message(&mut session, "assistant", &cleaned);
                cleaned
            }
            Ok(TaskResult::Failure { error, details }) => {
                let mut ea = FluentArgs::new();
                ea.set("err", error.to_string());
                let mut reply = tr_args("wx-task-fail", &ea);
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
                let t = sanitize_wechat_reply_output(&format!("{}\n{}", success, remaining));
                add_chat_message(&mut session, "assistant", &t);
                t
            }
            Err(e) => {
                let mut ea = FluentArgs::new();
                ea.set("err", e.to_string());
                tr_args("wx-exec-error", &ea)
            }
        };
        session.state = SessionState::Idle;
        let _ = save_session(&data_root, &account_id, &session);
        drop(session);

        for chunk in split_message(&reply, CHUNK_MAX) {
            let _ = sender
                .send_text(&from_user_id, &context_token, &chunk)
                .await;
        }
        let _ = active_task.lock().await.take();
    });

    *st.active_task.lock().await = Some(h);
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
    let mut sections = vec![format!(
        "## Channel Runtime\nchannel={channel_id}\nruntime_mode={runtime_mode}\ndefault_channel_agent={channel_default_agent}\nFor WeChat channel mode, default to workspace-assistant behavior. Only perform direct coding when the user explicitly asks to modify code."
    )];
    sections.push(
        "## WeChat 输出契约（必须遵守）\n\
         - 用户只看到你这一条最终回复。禁止输出：思考过程、自我解说（如「太好了」「我需要…」「从页面中可以提取」「根据系统提示我应该…」）、工具调用是否成功的说明、同一段数据的重复粘贴。\n\
         - 工具/Web 结果只内化进答案：用一小段话或一层列表给出结论即可；不要先写长段再抄一遍字段。\n\
         - 天气/事实类：几句带关键数字即可，不要「摘要 + 再列一遍同样指标」。\n\
         - English: Reply with the final answer only. No narration of your plan, no 'Great, fetch succeeded', no duplicate blocks.\n\
         - 禁止输出 <thought>、<thinking> 等标签或任何「推理草稿」块；只输出给用户的一句话/一小段结论。"
            .to_string(),
    );
    if let Some(existing) = existing {
        if !existing.trim().is_empty() {
            sections.push(existing.trim().to_string());
        }
    }
    sections.join("\n\n")
}

/// 去掉常见「废话」行（模型仍可能漏网，主要靠 system 约束）。
fn sanitize_wechat_reply_output(text: &str) -> String {
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
    let mut s = kept.join("\n");
    while s.contains("\n\n\n") {
        s = s.replace("\n\n\n", "\n\n");
    }
    s = s.trim().to_string();
    while s.contains("。。") {
        s = s.replace("。。", "。");
    }
    s
}

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
}
