use super::*;

fn truncate_field(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect()
}

pub async fn start_project_conversation(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<crate::schema::StartConversationRequest>,
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
        kind: body.kind.clone(),
        goal: body.goal.clone(),
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

    let project = match state.db.get_project(&project_id).await {
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

    let prompt = body.prompt.trim();
    let prompt_for_chat = crate::task_trigger::prompt_with_skills(prompt, body.skills.as_deref());
    let title = body
        .title
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| truncate_field(s, 120))
        .unwrap_or_else(|| truncate_field(prompt, 120));
    let prompt_preview = truncate_field(prompt, 240);
    let agent_type = body
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);

    let root_path = std::path::PathBuf::from(&project.root_path);
    let (root, _created_root) = match super::chat_util::ensure_chat_project_root(
        &state.db,
        &project_id,
        None,
        &root_path,
        "conversation_start",
    )
    .await
    {
        Ok(v) => v,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
        }
    };

    let kind = "repl";
    let session = match state
        .db
        .create_planned_session(CreateSessionRequest {
            project_id: project_id.clone(),
            kind: kind.to_string(),
            task_id: None,
            title: title.clone(),
            prompt_preview: Some(prompt_preview.clone()),
            agent_type: agent_type.clone(),
            model: None,
            metadata_json: Some(r#"{"source":"conversations_start"}"#.to_string()),
        })
        .await
    {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    let _ = state
        .db
        .insert_event(InsertEventRequest {
            project_id: project_id.clone(),
            session_id: Some(session.id.clone()),
            task_id: None,
            agent_id: None,
            event_type: "user_prompt".into(),
            severity: Some("info".into()),
            title: "User prompt".into(),
            body: Some(truncate_field(prompt, 8000)),
            payload: None,
        })
        .await;

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
            &session.id,
            &root,
            agent_type.as_deref(),
            &dashboard_url,
            &prompt_for_chat,
            body.vision_images.as_deref(),
        )
        .await
    {
        Ok(chat) => {
            let _ = state
                .db
                .merge_session_metadata(
                    &session.id,
                    &json!({
                        "web_chat": true,
                        "web_chat_log_path": chat.log_path,
                        "web_chat_pid": chat.pid,
                    }),
                )
                .await;
            let _ = crate::audit::record_audit(
                &state.db,
                crate::audit::AuditEventInput {
                    project_id: Some(project_id.clone()),
                    session_id: Some(session.id.clone()),
                    action: "conversation_started".into(),
                    risk: "medium".into(),
                    detail: json!({
                        "web_chat": true,
                        "pid": chat.pid,
                    }),
                },
            )
            .await;
            Json(json!({ "session": session, "chat": chat })).into_response()
        }
        Err(e) => {
            let _ = state
                .db
                .finish_session(
                    &session.id,
                    "failed",
                    Some(&format!("Failed to start task: {e}")),
                )
                .await;
            (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string(), "session_id": session.id })),
            )
                .into_response()
        }
    }
}
