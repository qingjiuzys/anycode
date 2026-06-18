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

pub async fn delete_cron_job(
    axum::extract::Path(job_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let job_id = job_id.trim();
    if job_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "job id is required" })),
        )
            .into_response();
    }
    let path = match cron_ledger::orchestration_path() {
        Some(p) => p,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "could not resolve orchestration path" })),
            )
                .into_response();
        }
    };
    match anycode_tools::remove_cron_job_from_orchestration_file(&path, job_id) {
        Ok(true) => Json(json!({ "ok": true, "job_id": job_id })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "cron job not found" })),
        )
            .into_response(),
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
                    project_id: j.project_id,
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

#[derive(Deserialize)]
pub struct CronRetryBody {
    pub job_id: String,
    pub project_id: Option<String>,
}

pub async fn retry_cron_job(
    State(state): State<AppState>,
    Json(body): Json<CronRetryBody>,
) -> impl IntoResponse {
    if !crate::task_trigger::triggers_allowed(&state.host) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "UI trigger run is disabled for this binding" })),
        )
            .into_response();
    }
    let job_id = body.job_id.trim();
    if job_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "job_id is required" })),
        )
            .into_response();
    }
    let jobs = match cron_ledger::read_cron_jobs(None) {
        Ok(j) => j,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
    let Some(job) = jobs.iter().find(|j| j.id == job_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "cron job not found" })),
        )
            .into_response();
    };
    let preferred_project = body.project_id.as_deref().or(job.project_id.as_deref());
    let project_id = match resolve_cron_retry_project(&state, preferred_project).await {
        Ok(id) => id,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };
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
    let req = crate::task_trigger::TriggerRunRequest {
        prompt: job.command.clone(),
        kind: "run".into(),
        goal: None,
        agent: Some("workspace-assistant".into()),
        skills: None,
    };
    match crate::task_trigger::trigger_run(
        &project_id,
        std::path::Path::new(&project.root_path),
        req,
        None,
        Some(&state.db),
    )
    .await
    {
        Ok(result) => Json(json!({
            "ok": true,
            "job_id": job_id,
            "trigger": result,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

async fn resolve_cron_retry_project(
    state: &AppState,
    preferred: Option<&str>,
) -> anyhow::Result<String> {
    if let Some(id) = preferred.map(str::trim).filter(|s| !s.is_empty()) {
        return Ok(id.to_string());
    }
    let projects = state.db.list_projects().await?;
    projects
        .first()
        .map(|p| p.id.clone())
        .ok_or_else(|| anyhow::anyhow!("no project available; pass project_id"))
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

#[derive(Deserialize)]
pub struct ParseCronScheduleBody {
    pub text: String,
}

pub async fn parse_cron_schedule(Json(body): Json<ParseCronScheduleBody>) -> impl IntoResponse {
    let text = body.text.trim();
    if text.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "text is required" })),
        )
            .into_response();
    }
    match anycode_tools::parse_natural_cron_hint(text) {
        Some(result) => {
            if let Err(e) = anycode_tools::validate_cron_schedule_expr(&result.schedule) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": format!("parsed schedule invalid: {e}") })),
                )
                    .into_response();
            }
            Json(json!({
                "ok": true,
                "schedule": result.schedule,
                "summary": result.summary,
            }))
            .into_response()
        }
        None => (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Could not parse schedule hint — try e.g. 每天8点, 每周五18:30, every day at 9am, or a 6-field cron"
            })),
        )
            .into_response(),
    }
}
