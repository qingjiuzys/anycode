use super::*;

pub async fn list_projects(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_projects().await {
        Ok(projects) => Json(json!({ "projects": projects })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_project(&project_id).await {
        Ok(Some(p)) => Json(json!({ "project": p })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "project not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn upsert_project(
    State(state): State<AppState>,
    Json(req): Json<UpsertProjectRequest>,
) -> impl IntoResponse {
    match state.db.upsert_project(req).await {
        Ok(p) => Json(json!({ "project": p })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn scan_projects(State(state): State<AppState>) -> impl IntoResponse {
    let paths = state.workspace_paths.clone();
    let registered = state.db.sync_workspace_paths(&paths).await.unwrap_or(0);
    let ingested = crate::ingest::ingest_recent_disk_tasks(&state.db, &state.tasks_root, &paths)
        .await
        .unwrap_or(0);
    let skills = skills_scan::sync_skills_to_db(&state.db, &paths)
        .await
        .unwrap_or(0);
    let _ = crate::audit::record_audit(
        &state.db,
        crate::audit::AuditEventInput::low(
            "projects_scan_requested",
            json!({
                "projects_registered": registered,
                "ingested_tasks": ingested,
                "skills_synced": skills,
            }),
        ),
    )
    .await;
    Json(json!({
        "ok": true,
        "projects_registered": registered,
        "ingested_tasks": ingested,
        "skills_synced": skills,
    }))
    .into_response()
}

pub async fn get_project_stats(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_project_stats(&project_id).await {
        Ok(stats) => Json(json!({ "stats": stats })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_project_data_health(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match crate::data_health::project_health(&state.db, &project_id).await {
        Ok(health) => Json(json!({ "health": health })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_project_metrics(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match crate::metrics::project_metrics(&state.db, &project_id).await {
        Ok(metrics) => Json(json!({ "metrics": metrics })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct TimelineQuery {
    #[serde(default = "default_timeline_days")]
    pub days: u32,
}

pub(super) fn default_timeline_days() -> u32 {
    7
}

pub async fn get_project_usage(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<TimelineQuery>,
) -> impl IntoResponse {
    match crate::metrics::project_token_usage_detail(&state.db, &project_id, q.days).await {
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

pub async fn trigger_project_run(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<crate::task_trigger::TriggerRunRequest>,
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
    let root = std::path::PathBuf::from(&project.root_path);
    match crate::task_trigger::trigger_run(&project_id, &root, body).await {
        Ok(trigger) => {
            let _ = crate::audit::record_audit(
                &state.db,
                crate::audit::AuditEventInput {
                    project_id: Some(project_id.clone()),
                    session_id: None,
                    action: "project_run_triggered".into(),
                    risk: "medium".into(),
                    detail: json!({
                        "trigger_id": trigger.trigger_id,
                        "kind": trigger.kind,
                        "pid": trigger.pid,
                        "command_preview": trigger.command_preview,
                    }),
                },
            )
            .await;
            Json(json!({ "trigger": trigger })).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_project_triggers(
    Path(project_id): Path<String>,
    Query(q): Query<LimitQuery>,
) -> impl IntoResponse {
    let triggers =
        crate::task_trigger::list_recent_triggers(&project_id, q.limit.clamp(1, 50) as usize);
    Json(json!({ "triggers": triggers })).into_response()
}
