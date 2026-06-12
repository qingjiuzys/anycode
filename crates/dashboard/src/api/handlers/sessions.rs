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
    let mut trigger_req = crate::task_trigger::TriggerRunRequest {
        prompt: body.prompt.clone(),
        kind: "run".into(),
        goal: None,
        agent: body.agent.clone(),
        skills: body.skills.clone(),
    };
    crate::task_trigger::normalize_trigger_request(&mut trigger_req);
    if let Err(e) = crate::task_trigger::validate_request(&trigger_req) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
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
    let effective_agent = requested_agent.or_else(|| {
        if session_agent.is_empty() {
            None
        } else {
            Some(session_agent)
        }
    });
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
    }
    let prompt_for_chat = crate::task_trigger::prompt_with_skills(prompt, body.skills.as_deref());
    if let Some(ref imgs) = body.vision_images {
        if let Err(e) = crate::control::vision_payload::validate_vision_payloads(imgs) {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    }
    let dashboard_url = super::chat_util::dashboard_loopback_url(&state.host, state.port);
    match state
        .web_chat
        .send(
            state.db.clone(),
            &session_id,
            &root,
            effective_agent,
            &dashboard_url,
            &prompt_for_chat,
            body.vision_images.as_deref(),
            body.text_files.as_deref(),
            body.lang.as_deref(),
        )
        .await
    {
        Ok(chat) => {
            let _ = state
                .db
                .insert_event(crate::schema::InsertEventRequest {
                    project_id: session.project_id.clone(),
                    session_id: Some(session_id.clone()),
                    task_id: None,
                    agent_id: None,
                    event_type: "user_prompt".into(),
                    severity: Some("info".into()),
                    title: "User prompt".into(),
                    body: Some(prompt.chars().take(8000).collect()),
                    payload: Some(json!({
                        "source": "web_chat",
                        "agent": requested_agent,
                        "skills": body.skills,
                    })),
                })
                .await;
            Json(json!({ "ok": true, "session_id": session_id, "chat": chat })).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string(), "session_id": session_id })),
        )
            .into_response(),
    }
}

pub async fn cancel_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let live_signal = crate::cancel_ipc::request_cancel(&session_id).unwrap_or(false);
    state.web_chat.evict(&session_id).await;
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
                        title: if live_signal {
                            "Session cancel signalled to CLI".into()
                        } else {
                            "Session cancelled from dashboard".into()
                        },
                        body: None,
                        payload: Some(json!({
                            "source": "dashboard",
                            "live_signal": live_signal
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
                "live_signal": live_signal
            }))
            .into_response()
        }
        Ok(false) => (
            StatusCode::CONFLICT,
            Json(json!({ "error": "session is not running" })),
        )
            .into_response(),
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
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    sse_filtered_events(state.events.subscribe(), None, Some(session_id))
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
        Ok(detail) => {
            Json(json!({ "usage": detail.usage, "by_model": detail.by_model })).into_response()
        }
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

#[derive(Deserialize)]
pub struct SecurityEventsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub project_id: Option<String>,
}
