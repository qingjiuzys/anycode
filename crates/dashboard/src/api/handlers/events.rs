use super::*;

pub async fn list_project_event_types(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match state.db.list_project_event_types(&project_id).await {
        Ok(types) => Json(json!({ "event_types": types })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_project_events(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<EventsQuery>,
) -> impl IntoResponse {
    match state
        .db
        .list_project_events(
            &project_id,
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

pub async fn publish_project_event(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(evt): Json<crate::schema::ProjectEvent>,
) -> impl IntoResponse {
    if evt.project_id != project_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "project_id mismatch" })),
        )
            .into_response();
    }
    state.events.publish(evt.clone());
    Json(json!({ "ok": true, "event": evt })).into_response()
}

pub async fn insert_project_event(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(mut req): Json<InsertEventRequest>,
) -> impl IntoResponse {
    req.project_id = project_id;
    match state.db.insert_event(req).await {
        Ok(evt) => {
            let evt_clone = evt.clone();
            state.events.publish(evt_clone);
            Json(json!({ "event": evt })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_recent_events(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    match state.db.list_recent_events(q.limit).await {
        Ok(events) => Json(json!({ "events": events })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_event(
    State(state): State<AppState>,
    Path(event_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_event(&event_id).await {
        Ok(Some(event)) => Json(json!({ "event": event })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "event not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn global_events_stream(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    sse_filtered_events(state.events.subscribe(), None, None)
}

pub async fn project_events_stream(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    sse_filtered_events(state.events.subscribe(), Some(project_id), None)
}

pub(super) fn sse_filtered_events(
    mut rx: tokio::sync::broadcast::Receiver<crate::schema::ProjectEvent>,
    project_filter: Option<String>,
    session_filter: Option<String>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let stream = stream! {
        yield Ok(Event::default().event("connected").data("{}"));
        loop {
            match rx.recv().await {
                Ok(evt)
                    if project_filter
                        .as_ref()
                        .is_none_or(|id| evt.project_id == *id)
                        && session_filter.as_ref().is_none_or(|sid| {
                            evt.session_id.as_deref() == Some(sid.as_str())
                        }) =>
                {
                    let data = serde_json::to_string(&evt).unwrap_or_default();
                    yield Ok(Event::default().event("project_event").data(data));
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
