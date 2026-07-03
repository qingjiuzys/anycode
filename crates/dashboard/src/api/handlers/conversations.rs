use super::*;

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

    if body.recycle_session {
        if let Ok(Some(recycled)) = state
            .db
            .find_recyclable_web_chat_session(&project_id, agent_type.as_deref())
            .await
        {
            let session_id = recycled.id.clone();
            if let Err(e) = state
                .db
                .reopen_session_for_chat(
                    &session_id,
                    Some(title.as_str()),
                    Some(prompt_preview.as_str()),
                )
                .await
            {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response();
            }
            let session_agent = recycled.agent_type.trim();
            if agent_type.as_deref().is_some() && agent_type.as_deref() != Some(session_agent) {
                if let Err(e) = state
                    .db
                    .update_session_agent(&session_id, agent_type.as_deref())
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
            match crate::control::web_chat_dispatch::dispatch_web_chat_prompt(
                &state,
                &project_id,
                &session_id,
                &root,
                agent_type.as_deref(),
                prompt,
                &prompt_for_chat,
                body.vision_images.as_deref(),
                body.text_files.as_deref(),
                body.lang.as_deref(),
                true,
                "conversation_recycled",
            )
            .await
            {
                Ok((session, chat)) => {
                    return Json(json!({
                        "session": session,
                        "chat": chat,
                        "recycled": true,
                    }))
                    .into_response();
                }
                Err((status, error)) => {
                    return (
                        status,
                        Json(json!({ "error": error, "session_id": session_id })),
                    )
                        .into_response();
                }
            }
        }
    }

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
                .into_response();
        }
    };

    match crate::control::web_chat_dispatch::dispatch_web_chat_prompt(
        &state,
        &project_id,
        &session.id,
        &root,
        agent_type.as_deref(),
        prompt,
        &prompt_for_chat,
        body.vision_images.as_deref(),
        body.text_files.as_deref(),
        body.lang.as_deref(),
        false,
        "conversation_started",
    )
    .await
    {
        Ok((session, chat)) => {
            Json(json!({ "session": session, "chat": chat, "recycled": false })).into_response()
        }
        Err((status, error)) => (
            status,
            Json(json!({ "error": error, "session_id": session.id })),
        )
            .into_response(),
    }
}

fn truncate_field(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect()
}
