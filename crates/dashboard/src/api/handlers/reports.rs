use super::*;

pub async fn get_project_report(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match crate::report::project_report(
        &state.db,
        &project_id,
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

pub async fn list_audit_events(
    State(state): State<AppState>,
    Query(q): Query<AuditQuery>,
) -> impl IntoResponse {
    match crate::audit::list_audit_events(
        &state.db,
        q.project_id.as_deref(),
        q.action.as_deref(),
        q.risk.as_deref(),
        q.limit,
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
