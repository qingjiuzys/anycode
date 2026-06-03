use super::*;

#[derive(Deserialize)]
pub struct ReportQuery {
    #[serde(default = "default_report_format")]
    pub format: String,
    #[serde(default = "default_report_events_limit")]
    pub events_limit: i64,
    #[serde(default = "default_report_artifacts_limit")]
    pub artifacts_limit: i64,
}

fn default_report_format() -> String {
    "json".into()
}

fn default_report_events_limit() -> i64 {
    50
}

fn default_report_artifacts_limit() -> i64 {
    30
}

impl ReportQuery {
    pub(crate) fn options(&self) -> crate::report::ReportOptions {
        crate::report::ReportOptions {
            events_limit: self.events_limit.clamp(1, 500),
            artifacts_limit: self.artifacts_limit.clamp(1, 200),
        }
    }
}

pub(crate) fn report_response(
    report: crate::schema::ReportDocument,
    format: &str,
) -> axum::response::Response {
    if format != "json" && format != "markdown" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "format must be json or markdown" })),
        )
            .into_response();
    }
    Json(json!({ "report": report })).into_response()
}

pub async fn get_project_report(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<ReportQuery>,
) -> impl IntoResponse {
    match crate::report::project_report(&state.db, &project_id, q.options(), true).await {
        Ok(report) => report_response(report, &q.format),
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
