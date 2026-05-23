use super::*;

pub async fn list_project_sessions(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    match state.db.list_sessions(&project_id, q.limit).await {
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
    match state.db.get_session(&session_id).await {
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

pub async fn cancel_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let live_signal = crate::cancel_ipc::request_cancel(&session_id).unwrap_or(false);
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
        .list_all_sessions(
            q.limit,
            kinds_ref,
            q.status.as_deref(),
            q.trusted_status.as_deref(),
            q.project_id.as_deref(),
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
) -> impl IntoResponse {
    match crate::report::session_report(
        &state.db,
        &session_id,
        crate::report::ReportOptions::default(),
        true,
    )
    .await
    {
        Ok(report) => Json(json!({ "report": report })).into_response(),
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

#[derive(Deserialize)]
pub struct SecurityEventsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub project_id: Option<String>,
}
