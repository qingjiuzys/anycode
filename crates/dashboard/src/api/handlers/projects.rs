use super::*;

#[derive(Deserialize)]
pub struct ProjectsQuery {
    #[serde(default = "default_projects_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub q: Option<String>,
    pub status: Option<String>,
    #[serde(default = "default_projects_sort")]
    pub sort: String,
}

fn default_projects_limit() -> i64 {
    100
}

fn default_projects_sort() -> String {
    "updated_at_desc".into()
}

pub async fn list_projects(
    State(state): State<AppState>,
    Query(q): Query<ProjectsQuery>,
) -> impl IntoResponse {
    match state
        .db
        .list_projects_paged(
            q.q.as_deref(),
            q.status.as_deref(),
            q.limit,
            q.offset,
            &q.sort,
        )
        .await
    {
        Ok((projects, total)) => Json(json!({
            "projects": projects,
            "total": total,
            "limit": q.limit.clamp(1, 500),
            "offset": q.offset.max(0),
        }))
        .into_response(),
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

fn builtin_project_template_summaries() -> Vec<crate::schema::ProjectTemplateSummary> {
    vec![crate::schema::ProjectTemplateSummary {
        id: "flutter-app".into(),
        name: "Flutter App".into(),
        name_zh: Some("Flutter 应用".into()),
        description: "Agent-first Flutter MVP with skills, gates, and goal workflow.".into(),
        description_zh: Some(
            "Agent 自主 Flutter：创建时仅骨架，由 Agent 安装 SDK 与平台目录。".into(),
        ),
        default_dir: "my_flutter_app".into(),
    }]
}

pub async fn list_project_templates() -> impl IntoResponse {
    let summaries = match anycode_tools::list_project_templates() {
        Ok(list) if !list.is_empty() => list
            .into_iter()
            .map(|t| crate::schema::ProjectTemplateSummary {
                id: t.id,
                name: t.name,
                name_zh: t.name_zh,
                description: t.description,
                description_zh: t.description_zh,
                default_dir: t.default_dir,
            })
            .collect(),
        _ => builtin_project_template_summaries(),
    };
    Json(json!({ "templates": summaries })).into_response()
}

pub async fn upsert_project(
    State(state): State<AppState>,
    Json(req): Json<UpsertProjectRequest>,
) -> impl IntoResponse {
    let root = req.root_path.trim();
    if root.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "root_path is required" })),
        )
            .into_response();
    }
    let template_id = req
        .template_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let root_path = if let Some(tid) = template_id.clone() {
        let target = std::path::PathBuf::from(root);
        let name = req.name.clone();
        let app_title = req.app_title.clone();
        let bundle_org = req.bundle_org.clone();
        let force = req.create_root.unwrap_or(true);
        match tokio::task::spawn_blocking(move || {
            anycode_tools::apply_project_template(
                &tid,
                &target,
                anycode_tools::ApplyTemplateOptions {
                    project_name: name,
                    app_title,
                    bundle_org,
                    force,
                    run_flutter_create: std::env::var_os("ANYCODE_TEMPLATE_RUN_FLUTTER_CREATE")
                        .is_some(),
                },
            )
        })
        .await
        {
            Ok(Ok(r)) => r.root,
            Ok(Err(e)) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response();
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response();
            }
        }
    } else {
        match crate::project_root::ensure_project_root(
            std::path::Path::new(root),
            req.create_root.unwrap_or(false),
        ) {
            Ok(path) => path,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response();
            }
        }
    };
    let root_display = root_path.display().to_string();
    let upsert = UpsertProjectRequest {
        root_path: root_display.clone(),
        name: req.name,
        description: req.description,
        create_root: req.create_root,
        template_id: req.template_id,
        app_title: req.app_title,
        bundle_org: req.bundle_org,
    };
    match state.db.upsert_project(upsert).await {
        Ok(p) => {
            let _ = skills_scan::sync_skills_to_db(&state.db, &[root_display]).await;
            Json(json!({ "project": p })).into_response()
        }
        Err(e) => {
            let msg = e.to_string();
            let status = if msg.contains("does not exist") || msg.contains("root_path") {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({ "error": msg }))).into_response()
        }
    }
}

pub async fn scan_projects(State(state): State<AppState>) -> impl IntoResponse {
    let paths = crate::workspace_index::collect_scan_workspace_paths();
    let registered = state.db.sync_workspace_paths(&paths).await.unwrap_or(0);
    let skills = skills_scan::sync_skills_to_db(&state.db, &paths)
        .await
        .unwrap_or(0);
    let _ = crate::audit::record_audit(
        &state.db,
        crate::audit::AuditEventInput::low(
            "projects_scan_requested",
            json!({
                "projects_registered": registered,
                "skills_synced": skills,
            }),
        ),
    )
    .await;
    Json(json!({
        "ok": true,
        "projects_registered": registered,
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

#[derive(Deserialize)]
pub struct PatchProjectRequest {
    pub name: String,
}

pub async fn patch_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<PatchProjectRequest>,
) -> impl IntoResponse {
    let name = body.name.trim();
    if name.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "name is required" })),
        )
            .into_response();
    }
    if name.chars().count() > 120 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "name must be at most 120 characters" })),
        )
            .into_response();
    }
    match state.db.rename_project(&project_id, name).await {
        Ok(true) => {
            Json(json!({ "ok": true, "project_id": project_id, "name": name })).into_response()
        }
        Ok(false) => (
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

#[derive(Deserialize)]
pub struct PatchProjectStatusRequest {
    pub status: String,
}

pub async fn patch_project_status(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<PatchProjectStatusRequest>,
) -> impl IntoResponse {
    let status = body.status.trim();
    if status.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "status is required" })),
        )
            .into_response();
    }
    match state.db.set_project_status(&project_id, status).await {
        Ok(true) => {
            Json(json!({ "ok": true, "project_id": project_id, "status": status })).into_response()
        }
        Ok(false) => (
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
    match crate::task_trigger::trigger_run(&project_id, &root, body, None, None).await {
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

pub async fn get_project_view_prefs(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_project_view_prefs(&project_id).await {
        Ok(Some(prefs)) => Json(json!({ "view_prefs": prefs.normalized() })).into_response(),
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

pub async fn put_project_view_prefs(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<crate::schema::ProjectViewPrefs>,
) -> impl IntoResponse {
    let prefs = body.normalized();
    match state.db.set_project_view_prefs(&project_id, &prefs).await {
        Ok(true) => Json(json!({ "ok": true, "view_prefs": prefs })).into_response(),
        Ok(false) => (
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
