use super::*;

#[derive(Deserialize)]
pub struct ReportQuery {
    #[serde(default = "default_report_format")]
    pub format: String,
    #[serde(default = "default_report_events_limit")]
    pub events_limit: i64,
    #[serde(default = "default_report_artifacts_limit")]
    pub artifacts_limit: i64,
    pub lang: Option<String>,
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
        let mut opts = crate::report::ReportOptions::from_lang_param(self.lang.as_deref());
        opts.events_limit = self.events_limit.clamp(1, 500);
        opts.artifacts_limit = self.artifacts_limit.clamp(1, 200);
        opts
    }
}

pub(crate) fn report_response(
    report: crate::schema::ReportDocument,
    format: &str,
) -> axum::response::Response {
    match format {
        "json" => Json(json!({ "report": report })).into_response(),
        "markdown" => (
            [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
            report.markdown.clone(),
        )
            .into_response(),
        "html" => {
            let body = report
                .html
                .clone()
                .unwrap_or_else(|| report.markdown.clone());
            ([(header::CONTENT_TYPE, "text/html; charset=utf-8")], body).into_response()
        }
        _ => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "format must be json, markdown, or html" })),
        )
            .into_response(),
    }
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
