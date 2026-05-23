use super::*;

pub async fn list_cron_runs(
    State(state): State<AppState>,
    Query(q): Query<CronRunsQuery>,
) -> impl IntoResponse {
    let limit = q.limit.max(1) as usize;
    match cron_ledger::read_cron_runs(limit, q.job_id.as_deref(), q.session_id.as_deref()) {
        Ok(rows) => {
            let mut out = Vec::with_capacity(rows.len());
            for r in rows {
                let dashboard_session_id = if r.session_id.is_empty() {
                    None
                } else {
                    state
                        .db
                        .find_session_by_correlation(&r.session_id)
                        .await
                        .ok()
                        .flatten()
                };
                out.push(CronRunRecord {
                    job_id: r.job_id,
                    session_id: r.session_id,
                    fired_at: r.fired_at,
                    status: r.status,
                    detail: r.detail,
                    line_no: r.line_no,
                    dashboard_session_id,
                });
            }
            Json(json!({ "runs": out, "ledger_path": cron_ledger::cron_runs_path().map(|p| p.display().to_string()) })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_cron_jobs(State(_state): State<AppState>) -> impl IntoResponse {
    match cron_ledger::read_cron_jobs(None) {
        Ok(jobs) => {
            let jobs: Vec<CronJobRecord> = jobs
                .into_iter()
                .map(|j| CronJobRecord {
                    id: j.id,
                    schedule: j.schedule,
                    command: j.command,
                    session_id: j.session_id,
                    failure_destination: j.failure_destination,
                    tool_profile: j.tool_profile,
                })
                .collect();
            Json(json!({
                "jobs": jobs,
                "orchestration_path": cron_ledger::orchestration_path().map(|p| p.display().to_string())
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

pub async fn list_agent_stats(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    match state.db.list_agent_usage_stats(q.limit).await {
        Ok(stats) => Json(json!({ "agents": stats })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_recent_reports(
    State(state): State<AppState>,
    Query(q): Query<RecentReportsQuery>,
) -> impl IntoResponse {
    match crate::report_archive::list_recent_reports(
        &state.db,
        q.project_id.as_deref(),
        q.session_id.as_deref(),
        q.limit.unwrap_or(20),
    )
    .await
    {
        Ok(reports) => Json(json!({ "reports": reports })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct RecentReportsQuery {
    pub project_id: Option<String>,
    pub session_id: Option<String>,
    pub limit: Option<i64>,
}

pub async fn get_delivery_readiness(State(state): State<AppState>) -> impl IntoResponse {
    match crate::metrics::global_readiness(&state.db).await {
        Ok(readiness) => Json(json!({ "readiness": readiness })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_usage_metrics(
    State(state): State<AppState>,
    Query(q): Query<TimelineQuery>,
) -> impl IntoResponse {
    match crate::metrics::global_token_usage_detail(&state.db, q.days).await {
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

pub async fn get_saved_hours_kpi(
    State(state): State<AppState>,
    Query(q): Query<TimelineQuery>,
) -> impl IntoResponse {
    match crate::metrics::saved_hours_kpi(&state.db, q.days).await {
        Ok(kpi) => Json(json!({ "kpi": kpi })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct UsageExportQuery {
    #[serde(default = "default_timeline_days")]
    pub days: u32,
    pub project_id: Option<String>,
}

pub async fn export_usage_metrics(
    State(state): State<AppState>,
    Query(q): Query<UsageExportQuery>,
) -> impl IntoResponse {
    match crate::metrics::usage_export_csv(&state.db, q.days, q.project_id.as_deref()).await {
        Ok(csv) => (
            [
                (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                (
                    header::CONTENT_DISPOSITION,
                    "attachment; filename=\"token-usage.csv\"",
                ),
            ],
            csv,
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_recent_notifications(
    State(state): State<AppState>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    let limit = q.limit.clamp(1, 100);
    match crate::audit::list_recent_notifications(&state.db, limit).await {
        Ok(notifications) => Json(json!({ "notifications": notifications })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_timeline_metrics(
    State(state): State<AppState>,
    Query(q): Query<TimelineQuery>,
) -> impl IntoResponse {
    match crate::metrics::global_timeline(&state.db, q.days).await {
        Ok(timeline) => Json(json!({ "timeline": timeline })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct PatchLlmConfigBody {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub fallback_provider: Option<String>,
    #[serde(default)]
    pub fallback_model: Option<String>,
}
