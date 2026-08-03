use super::*;

pub async fn list_project_sessions(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    match state.db.list_sessions_enriched(&project_id, q.limit).await {
        Ok(sessions) => Json(json!({ "sessions": sessions })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_session_enriched(&session_id).await {
        Ok(Some(s)) => Json(json!({ "session": s })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "session not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct PatchSessionRequest {
    pub title: String,
}

pub async fn patch_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<PatchSessionRequest>,
) -> impl IntoResponse {
    let title = body.title.trim();
    if title.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "title is required" })),
        )
            .into_response();
    }
    if title.chars().count() > 120 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "title must be at most 120 characters" })),
        )
            .into_response();
    }
    match state.db.get_session(&session_id).await {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "session not found" })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
        Ok(Some(_)) => {}
    }
    match state
        .db
        .update_session_metadata(&session_id, Some(title), None)
        .await
    {
        Ok(()) => {
            Json(json!({ "ok": true, "session_id": session_id, "title": title })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn send_session_message(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<crate::schema::SendConversationMessageRequest>,
) -> impl IntoResponse {
    if !crate::task_trigger::triggers_allowed(&state.host) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "UI trigger run is disabled for this binding. Use loopback or set ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE=1."
            })),
        )
            .into_response();
    }
    let vision_count = body.vision_images.as_ref().map_or(0, |v| v.len());
    let text_file_count = body.text_files.as_ref().map_or(0, |f| f.len());
    if let Err(e) = crate::task_trigger::validate_conversation_message(
        &body.prompt,
        vision_count,
        text_file_count,
    ) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
    if let Err(e) = crate::task_trigger::validate_skill_ids(body.skills.as_deref()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
    if let Some(agent) = body
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        if agent.len() > 64
            || !agent
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "invalid agent id" })),
            )
                .into_response();
        }
    }
    let prompt = body.prompt.trim();
    let session = match state.db.get_session(&session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "session not found" })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    let project = match state.db.get_project(&session.project_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "project not found" })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    let root_path = std::path::PathBuf::from(&project.root_path);
    if let Err(e) = crate::task_trigger::validate_trigger_skills_for_project(
        body.skills.as_deref(),
        body.agent.as_deref(),
        &root_path,
    )
    .await
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
    let (root, _created_root) = match super::chat_util::ensure_chat_project_root(
        &state.db,
        &session.project_id,
        Some(&session_id),
        &root_path,
        "conversation_message",
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
        }
    };
    let requested_agent = body
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let session_agent = session.agent_type.trim();
    let effective_agent = requested_agent.or({
        if session_agent.is_empty() {
            None
        } else {
            Some(session_agent)
        }
    });
    let resolved_agent = crate::control::agent_resolve::resolve_web_chat_agent(effective_agent);
    if requested_agent.is_some() && requested_agent != Some(session_agent) {
        if let Err(e) = state
            .db
            .update_session_agent(&session_id, requested_agent)
            .await
        {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
        state.web_chat.evict(&session_id).await;
        state.chat_runtime.evict(&session_id).await;
    }
    if let Some(ref imgs) = body.vision_images {
        if let Err(e) = crate::control::vision_payload::validate_vision_payloads(imgs) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
        if !imgs.is_empty() {
            match crate::control::vision_payload::can_accept_images_for_chat() {
                Ok(false) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({
                            "error": "Active chat model does not support vision, and OCR is unavailable. Enable Apple OCR on Desktop, or switch chat to a vision-capable model."
                        })),
                    )
                        .into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": format!("vision capability check failed: {e}") })),
                    )
                        .into_response();
                }
                Ok(true) => {}
            }
        }
    }
    if let Some(ref files) = body.text_files {
        if let Err(e) = crate::control::text_upload::validate_text_payloads(files) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    }

    let should_enqueue = body.enqueue.unwrap_or(true)
        && crate::control::message_queue::session_accepts_enqueue(
            &state,
            &session_id,
            &session.status,
        )
        .await;
    if should_enqueue {
        if !crate::question_ipc::list_pending_for_session(Some(&session_id), 1).is_empty() {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "error": "session has pending AskUserQuestion",
                    "session_id": session_id,
                })),
            )
                .into_response();
        }
        match state
            .db
            .enqueue_session_message(crate::db::EnqueueMessageInput {
                session_id: session_id.clone(),
                prompt: prompt.to_string(),
                agent: requested_agent.map(str::to_string),
                skills: body.skills.clone(),
                vision_images: body.vision_images.clone(),
                text_files: body.text_files.clone(),
                lang: body.lang.clone(),
                composer_mode: body.composer_mode.clone(),
            })
            .await
        {
            Ok((queued, position)) => {
                if let Ok(evt) = state
                    .db
                    .insert_message_queued_event(
                        &session.project_id,
                        &session_id,
                        &queued.id,
                        position,
                        prompt,
                    )
                    .await
                {
                    crate::control::web_chat_tail::publish_project_chat_event(&state.events, &evt);
                }
                if !state.chat_runtime.is_turn_in_flight(&session_id).await {
                    crate::control::message_queue::spawn_drain_if_idle(&state, &session_id);
                }
                return (
                    StatusCode::ACCEPTED,
                    Json(json!({
                        "ok": true,
                        "queued": true,
                        "queue_id": queued.id,
                        "position": position,
                        "session_id": session_id,
                        "item": queued,
                    })),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response();
            }
        }
    }

    let prompt_for_chat = crate::task_trigger::prompt_with_skills(prompt, body.skills.as_deref());
    match crate::control::web_chat_dispatch::dispatch_web_chat_prompt(
        &state,
        &session.project_id,
        &session_id,
        &root,
        Some(resolved_agent.as_str()),
        prompt,
        &prompt_for_chat,
        body.vision_images.as_deref(),
        body.text_files.as_deref(),
        body.lang.as_deref(),
        false,
        "conversation_message",
        body.composer_mode.as_deref(),
    )
    .await
    {
        Ok((session, chat)) => {
            Json(json!({ "ok": true, "session": session, "session_id": session_id, "chat": chat }))
                .into_response()
        }
        Err((status, error)) => (
            status,
            Json(json!({ "error": error, "session_id": session_id })),
        )
            .into_response(),
    }
}

pub async fn list_session_message_queue(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_session(&session_id).await {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "session not found" })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
        Ok(Some(_)) => {}
    }
    match state.db.list_pending_session_messages(&session_id).await {
        Ok(items) => Json(json!({ "items": items, "session_id": session_id })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn cancel_session_message_queue_item(
    State(state): State<AppState>,
    Path((session_id, queue_id)): Path<(String, String)>,
) -> impl IntoResponse {
    match state.db.cancel_queued_message(&session_id, &queue_id).await {
        Ok(true) => Json(json!({ "ok": true, "session_id": session_id, "queue_id": queue_id }))
            .into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "queued message not found or not pending" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn cancel_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let live_signal = crate::cancel_ipc::request_cancel(&session_id).unwrap_or(false);
    // Unblock wait_web loops that only poll approval/question IPC.
    let _ = crate::approval_ipc::clear_pending_for_session(&session_id);
    let _ = crate::question_ipc::clear_pending_for_session(&session_id);
    state.web_chat.evict(&session_id).await;
    // Signal the embedded turn directly (no IPC poll latency). The session
    // object stays alive so the running turn unwinds under its own epoch;
    // do NOT evict here, or the stale turn loses its epoch guard.
    let embedded_signal = state.chat_runtime.cancel(&session_id).await;
    let _ = state
        .db
        .cancel_all_pending_queue_messages(&session_id)
        .await;
    match state.db.cancel_running_session(&session_id).await {
        Ok(true) => {
            if let Ok(Some(sess)) = state.db.get_session(&session_id).await {
                let _ = state
                    .db
                    .insert_event(crate::schema::InsertEventRequest {
                        project_id: sess.project_id.clone(),
                        session_id: Some(session_id.clone()),
                        task_id: None,
                        agent_id: None,
                        event_type: "session_cancelled".into(),
                        severity: Some("warn".into()),
                        title: if live_signal || embedded_signal {
                            "Session cancel signalled to CLI".into()
                        } else {
                            "Session cancelled from dashboard".into()
                        },
                        body: None,
                        payload: Some(json!({
                            "source": "dashboard",
                            "live_signal": live_signal,
                            "embedded_signal": embedded_signal
                        })),
                    })
                    .await;
                let _ = crate::audit::record_audit(
                    &state.db,
                    crate::audit::AuditEventInput {
                        project_id: Some(sess.project_id.clone()),
                        session_id: Some(session_id.clone()),
                        action: "session_cancelled".into(),
                        risk: "medium".into(),
                        detail: json!({ "source": "dashboard", "live_signal": live_signal }),
                    },
                )
                .await;
            }
            Json(json!({
                "ok": true,
                "session_id": session_id,
                "live_signal": live_signal,
                "embedded_signal": embedded_signal
            }))
            .into_response()
        }
        Ok(false) => {
            // Idempotent: session already terminal (turn finished but UI may still
            // show running). Return success so the client can refresh and unstick.
            Json(json!({
                "ok": true,
                "session_id": session_id,
                "already_idle": true,
                "live_signal": live_signal,
                "embedded_signal": embedded_signal,
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn acknowledge_session_block(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.db.acknowledge_session_block(&session_id).await {
        Ok(true) => {
            if let Ok(Some(sess)) = state.db.get_session(&session_id).await {
                let _ = crate::audit::record_audit(
                    &state.db,
                    crate::audit::AuditEventInput {
                        project_id: Some(sess.project_id.clone()),
                        session_id: Some(session_id.clone()),
                        action: "session_block_acknowledged".into(),
                        risk: "low".into(),
                        detail: json!({ "source": "dashboard" }),
                    },
                )
                .await;
            }
            Json(json!({ "ok": true, "session_id": session_id })).into_response()
        }
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "session not found or not blocked" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_session_auto_approve(Path(session_id): Path<String>) -> impl IntoResponse {
    Json(json!({
        "session_id": session_id,
        "enabled": crate::approval_ipc::session_auto_approve_enabled(&session_id),
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct AutoApproveBody {
    pub enabled: bool,
}

pub async fn set_session_auto_approve(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(body): Json<AutoApproveBody>,
) -> impl IntoResponse {
    if !crate::approval_ipc::respond_allowed(&state.host) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Web approval respond is disabled for this binding. Use loopback or set ANYCODE_DASHBOARD_WEB_APPROVAL_REMOTE=1."
            })),
        )
            .into_response();
    }
    if let Err(e) = crate::approval_ipc::set_session_auto_approve(&session_id, body.enabled) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
    let _ = crate::audit::record_audit(
        &state.db,
        crate::audit::AuditEventInput {
            project_id: None,
            session_id: Some(session_id.clone()),
            action: "session_auto_approve_toggled".into(),
            risk: "medium".into(),
            detail: json!({ "enabled": body.enabled, "source": "dashboard" }),
        },
    )
    .await;
    Json(json!({ "ok": true, "session_id": session_id, "enabled": body.enabled })).into_response()
}

pub async fn list_all_sessions(
    State(state): State<AppState>,
    Query(q): Query<SessionsQuery>,
) -> impl IntoResponse {
    let kinds: Option<Vec<String>> = q.kind.as_ref().map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|k| !k.is_empty())
            .map(str::to_string)
            .collect()
    });
    let kinds_ref = kinds.as_deref();
    match state
        .db
        .list_all_sessions_enriched(
            q.limit,
            kinds_ref,
            q.status.as_deref(),
            q.trusted_status.as_deref(),
            q.project_id.as_deref(),
            q.budget_exceeded.unwrap_or(false),
        )
        .await
    {
        Ok(sessions) => Json(json!({ "sessions": sessions })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_session_facets(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.session_facets().await {
        Ok(facets) => Json(json!({ "facets": facets })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> impl IntoResponse {
    match state.db.create_session(req).await {
        Ok(s) => Json(json!({ "session": s })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_session_events(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    match state
        .db
        .list_session_events(
            &session_id,
            q.after.as_deref(),
            q.limit,
            q.event_type.as_deref(),
            q.severity.as_deref(),
            q.q.as_deref(),
        )
        .await
    {
        Ok(events) => Json(json!({ "events": events })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_session_event_types(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.db.list_session_event_types(&session_id).await {
        Ok(types) => Json(json!({ "event_types": types })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn session_events_stream(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(q): Query<SessionStreamQuery>,
    headers: axum::http::HeaderMap,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let after_seq = q.after_seq.or_else(|| {
        headers
            .get("Last-Event-ID")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<i64>().ok())
    });
    sse_session_stream(
        state.db.clone(),
        state.events.subscribe(),
        state.events.subscribe_chat(),
        session_id,
        after_seq,
    )
}

pub async fn list_session_gates(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.db.list_gates_for_session(&session_id).await {
        Ok(gates) => Json(json!({ "gates": gates })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_running_sessions(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    match state.db.list_running_sessions(q.limit).await {
        Ok(sessions) => Json(json!({ "sessions": sessions })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_session_report(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(q): Query<super::reports::ReportQuery>,
) -> impl IntoResponse {
    match crate::report::session_report(&state.db, &session_id, q.options(), true).await {
        Ok(report) => super::reports::report_response(report, &q.format),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_session_replay(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match crate::session_replay::session_replay(&state.db, &session_id).await {
        Ok(replay) => Json(json!({ "replay": replay })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_session_trace(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match crate::session_trace::session_trace(&state.db, &session_id).await {
        Ok(trace) => Json(json!({ "trace": trace })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_session_transcript(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match crate::session_transcript::session_transcript(&state.db, &session_id).await {
        Ok(transcript) => Json(json!({ "transcript": transcript })).into_response(),
        Err(e) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct ExecutionLogQuery {
    #[serde(default)]
    pub offset: usize,
    #[serde(default = "default_execution_log_limit")]
    pub limit: usize,
}

fn default_execution_log_limit() -> usize {
    200
}

pub async fn get_session_execution_log(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(q): Query<ExecutionLogQuery>,
) -> impl IntoResponse {
    match state.db.get_session(&session_id).await {
        Ok(Some(session)) => {
            match crate::execution_log::read_execution_log_async(session, q.offset, Some(q.limit))
                .await
            {
                Ok(log) => Json(json!({ "execution_log": log })).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response(),
            }
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "session not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_session_usage(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match crate::metrics::session_token_usage_detail(&state.db, &session_id).await {
        Ok(detail) => Json(json!({
            "usage": detail.usage,
            "by_model": detail.by_model,
            "by_project": detail.by_project,
            "by_day": detail.by_day,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_session_background_tasks(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let session = match state.db.get_session(&session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "session not found" })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    let mut orchestration_tasks = Vec::new();
    if let Some(path) = cron_ledger::orchestration_path() {
        if path.is_file() {
            if let Ok(raw) = std::fs::read_to_string(&path) {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                    if let Some(tasks) = v.get("tasks").and_then(|t| t.as_object()) {
                        for (id, rec) in tasks {
                            let meta = rec.get("metadata").cloned().unwrap_or(json!({}));
                            let session_match = meta
                                .get("session_id")
                                .and_then(|x| x.as_str())
                                .is_some_and(|sid| sid == session_id);
                            let task_match = session
                                .task_id
                                .as_deref()
                                .is_some_and(|tid| tid == id.as_str());
                            if session_match || task_match {
                                orchestration_tasks.push(json!({
                                    "id": id,
                                    "subject": rec.get("subject"),
                                    "status": rec.get("status"),
                                    "description": rec.get("description"),
                                }));
                            }
                        }
                    }
                }
            }
        }
    }

    let mut agent_tool_calls = Vec::new();
    if let Ok(events) = state
        .db
        .list_session_events(&session_id, None, 200, Some("tool_call_end"), None, None)
        .await
    {
        for e in events {
            let name = e.payload.get("name").and_then(|v| v.as_str()).unwrap_or("");
            if matches!(
                name,
                "Agent" | "Task" | "TaskCreate" | "TaskOutput" | "TaskStop"
            ) {
                agent_tool_calls.push(json!({
                    "occurred_at": e.occurred_at,
                    "title": e.title,
                    "severity": e.severity,
                    "tool": name,
                    "body": e.body,
                }));
            }
        }
    }

    Json(json!({
        "orchestration_tasks": orchestration_tasks,
        "agent_tool_calls": agent_tool_calls,
    }))
    .into_response()
}

pub async fn get_session_plan_tree(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_session(&session_id).await {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "session not found" })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
        Ok(Some(_)) => {}
    }
    match state.db.get_session_plan_tree(&session_id).await {
        Ok(Some((tree, updated_at))) => Json(json!({
            "tree": tree,
            "updated_at": updated_at,
        }))
        .into_response(),
        Ok(None) => Json(json!({
            "tree": { "roots": [] },
            "updated_at": null,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_session_plan_tree(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_session(&session_id).await {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "session not found" })),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
        Ok(Some(_)) => {}
    }
    match state.db.delete_session_plan_tree(&session_id).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct SecurityEventsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub project_id: Option<String>,
}
