use super::*;

#[derive(Deserialize)]
pub struct ProjectKnowledgePathsBody {
    pub paths: Vec<String>,
}

async fn project_root(
    state: &AppState,
    project_id: &str,
) -> Result<String, (StatusCode, Json<serde_json::Value>)> {
    match state.db.get_project(project_id).await {
        Ok(Some(p)) => Ok(p.root_path),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "project not found" })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )),
    }
}

pub async fn get_project_knowledge(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = project_root(&state, &project_id).await {
        return e.into_response();
    }
    match crate::project_knowledge::list_paths(&state.db, &project_id).await {
        Ok(paths) => Json(json!({ "paths": paths })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn put_project_knowledge(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<ProjectKnowledgePathsBody>,
) -> impl IntoResponse {
    if let Err(e) = project_root(&state, &project_id).await {
        return e.into_response();
    }
    match crate::project_knowledge::set_paths(&state.db, &project_id, &body.paths).await {
        Ok(()) => Json(json!({ "ok": true, "paths": body.paths })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn reindex_project_knowledge(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    let root = match project_root(&state, &project_id).await {
        Ok(r) => r,
        Err(e) => return e.into_response(),
    };
    match crate::project_knowledge::reindex_project(&state.db, std::path::Path::new(&root)).await {
        Ok(chunks) => Json(json!({ "ok": true, "chunks_indexed": chunks })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn search_project_knowledge(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<KnowledgeSearchQuery>,
) -> impl IntoResponse {
    let root = match project_root(&state, &project_id).await {
        Ok(r) => r,
        Err(e) => return e.into_response(),
    };
    let limit = q.limit.unwrap_or(8).clamp(1, 20) as usize;
    match crate::project_knowledge::search(&state.db, std::path::Path::new(&root), &q.q, limit)
        .await
    {
        Ok(hits) => Json(json!({ "hits": hits })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct KnowledgeSearchQuery {
    pub q: String,
    pub limit: Option<i64>,
}

pub async fn get_project_knowledge_stats(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = project_root(&state, &project_id).await {
        return e.into_response();
    }
    match crate::project_knowledge::stats(&state.db, &project_id).await {
        Ok(stats) => Json(json!({ "stats": stats })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct ImportSkillBody {
    pub source: String,
}

pub async fn import_skill(
    State(state): State<AppState>,
    Json(body): Json<ImportSkillBody>,
) -> impl IntoResponse {
    let Some(home) = dirs::home_dir() else {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": "no home directory" })),
        )
            .into_response();
    };
    let dest = home.join(".anycode/skills");
    match anycode_tools::install_skill(body.source.trim(), &dest) {
        Ok(r) => {
            let _ = crate::skills_scan::sync_skills_to_db(&state.db, &state.workspace_paths).await;
            Json(json!({
                "ok": true,
                "id": r.id,
                "path": r.dest.display().to_string(),
            }))
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateCronJobBody {
    pub schedule: String,
    pub command: String,
    #[serde(default = "default_cron_tz")]
    pub schedule_timezone: String,
    pub session_id: Option<String>,
    pub failure_destination: Option<String>,
    pub tool_profile: Option<String>,
    #[serde(default)]
    pub project_id: Option<String>,
}

fn default_cron_tz() -> String {
    "local".to_string()
}

pub async fn create_cron_job(Json(body): Json<CreateCronJobBody>) -> impl IntoResponse {
    if let Err(e) = anycode_tools::validate_cron_schedule_expr(&body.schedule) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": format!("invalid schedule: {e}") })),
        )
            .into_response();
    }
    let tz = match anycode_tools::resolve_schedule_timezone(body.schedule_timezone.trim()) {
        Ok(t) => t,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": e }))).into_response();
        }
    };
    let schedule =
        match anycode_tools::wall_clock_cron_to_utc_storage_for_timezone(&body.schedule, tz) {
            Some(s) => s,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": "could not convert schedule to UTC storage" })),
                )
                    .into_response();
            }
        };
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
    let opts = anycode_tools::CronJobCreateOptions {
        session_id: body.session_id,
        failure_destination: body.failure_destination,
        tool_profile: body.tool_profile,
        tool_allowlist: None,
        project_id: body.project_id,
    };
    match anycode_tools::append_cron_job_to_orchestration_file(&path, schedule, body.command, opts)
    {
        Ok(job) => Json(json!({ "ok": true, "job": job })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_orchestration_tasks() -> impl IntoResponse {
    let path = cron_ledger::orchestration_path();
    let Some(path) = path else {
        return Json(json!({ "tasks": {}, "teams": {} })).into_response();
    };
    if !path.is_file() {
        return Json(json!({ "tasks": {}, "teams": {} })).into_response();
    }
    let raw = match std::fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    let v: serde_json::Value = serde_json::from_str(&raw).unwrap_or(json!({}));
    Json(json!({
        "tasks": v.get("tasks").cloned().unwrap_or(json!({})),
        "teams": v.get("teams").cloned().unwrap_or(json!({})),
        "orchestration_path": path.display().to_string(),
    }))
    .into_response()
}

pub async fn list_automation_templates() -> impl IntoResponse {
    let mut templates = Vec::new();
    let candidates = [
        std::path::PathBuf::from("automation-templates"),
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../automation-templates"),
    ];
    for dir in candidates {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for ent in entries.flatten() {
                if ent.path().extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                if let Ok(text) = std::fs::read_to_string(ent.path()) {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                        templates.push(v);
                    }
                }
            }
        }
        break;
    }
    Json(json!({ "templates": templates })).into_response()
}
