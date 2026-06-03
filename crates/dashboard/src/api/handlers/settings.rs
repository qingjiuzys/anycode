use super::*;

pub async fn list_services(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.list_local_services().await {
        Ok(rows) => {
            let services: Vec<LocalServiceRecord> = rows
                .into_iter()
                .map(|(name, host, port, status, auth_mode)| LocalServiceRecord {
                    name,
                    host,
                    port: port as u16,
                    status,
                    auth_mode,
                })
                .collect();
            Json(json!({ "services": services })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn database_settings(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "path": state.db.path().display().to_string(),
        "driver": "sqlite"
    }))
}

#[derive(Deserialize)]
pub struct CronRunsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    pub job_id: Option<String>,
    pub session_id: Option<String>,
}

pub async fn get_policy_summary(State(state): State<AppState>) -> impl IntoResponse {
    let policy = crate::audit::policy_summary(&state.host, state.port);
    Json(json!({ "policy": policy }))
}

pub async fn get_data_health(State(state): State<AppState>) -> impl IntoResponse {
    match crate::data_health::global_health(&state.db).await {
        Ok(health) => Json(json!({ "health": health })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_service_status(State(state): State<AppState>) -> impl IntoResponse {
    let status = crate::service_governance::build_service_status(
        &state.host,
        state.port,
        &state.version,
        state.db.path(),
        state.static_dir.as_deref(),
        &state.started_at,
        state.pid,
        state.events.subscriber_count(),
        state.events.last_event_at().as_deref(),
    );
    Json(json!({ "service": status })).into_response()
}

pub async fn get_doctor(State(state): State<AppState>) -> impl IntoResponse {
    let mut report = crate::service_governance::run_doctor_checks(
        &state.host,
        state.port,
        state.db.path(),
        state.static_dir.as_deref(),
    );
    report
        .checks
        .extend(crate::service_governance::llm_doctor_checks());
    report
        .checks
        .extend(crate::connector_health::connector_doctor_checks(&state.db).await);
    report
        .checks
        .extend(crate::governance::workbench_doctor::workbench_doctor_checks(&state.db).await);
    report.status = crate::service_governance::doctor_overall_status(&report.checks).into();
    let has_projects = state
        .db
        .overview_stats()
        .await
        .map(|s| s.projects_count > 0)
        .unwrap_or(false);
    let active_tokens = crate::tokens::token_count_active(&state.db)
        .await
        .unwrap_or(0);
    let loopback = crate::service_governance::is_loopback_host(&state.host);
    report.next_steps = crate::service_governance::doctor_next_steps(
        &report,
        has_projects,
        active_tokens,
        loopback,
    );
    Json(json!({ "doctor": report })).into_response()
}

pub async fn get_runtime_settings(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state
        .db
        .overview_stats()
        .await
        .unwrap_or(crate::schema::OverviewStats {
            projects_count: 0,
            sessions_total: 0,
            sessions_running: 0,
            sessions_blocked: 0,
            sessions_budget_exceeded: 0,
            artifacts_count: 0,
            skills_count: 0,
            gates_failed: 0,
            events_last_hour: 0,
        });
    let enabled_links = state.db.project_skill_enabled_count().await.unwrap_or(0);
    let saved_prefs = crate::preferences::load_preferences();
    let runtime = crate::runtime_config::build_runtime_settings(
        &state.host,
        state.port,
        state.db.path(),
        stats.skills_count,
        enabled_links,
        saved_prefs.as_ref(),
    );
    Json(json!({ "runtime": runtime })).into_response()
}

fn active_preferences(state: &AppState) -> crate::schema::DashboardPreferences {
    let mut prefs = crate::schema::DashboardPreferences {
        host: state.host.clone(),
        port: state.port,
        db_path: state.db.path().display().to_string(),
        asset_read_strict: false,
        updated_at: state.started_at.clone(),
    };
    if let Some(saved) = crate::preferences::load_preferences() {
        prefs.asset_read_strict = saved.asset_read_strict;
    }
    prefs
}

pub async fn get_dashboard_preferences(State(state): State<AppState>) -> impl IntoResponse {
    let active = active_preferences(&state);
    let saved = crate::preferences::load_preferences();
    let restart_required = saved.as_ref().is_some_and(|s| {
        s.host != active.host || s.port != active.port || s.db_path != active.db_path
    });
    let restart_host = saved
        .as_ref()
        .map(|s| s.host.as_str())
        .unwrap_or(&active.host);
    let restart_port = saved.as_ref().map(|s| s.port).unwrap_or(active.port);
    let restart_db = saved
        .as_ref()
        .map(|s| s.db_path.as_str())
        .unwrap_or(active.db_path.as_str());
    let view = crate::schema::DashboardPreferencesView {
        active: active.clone(),
        saved: saved.clone(),
        restart_command: crate::preferences::restart_command(
            restart_host,
            restart_port,
            std::path::Path::new(restart_db),
        ),
        preferences_path: crate::preferences::preferences_path().display().to_string(),
        restart_required,
    };
    Json(json!({ "preferences": view })).into_response()
}

#[derive(Deserialize)]
pub struct PutDashboardPreferences {
    pub host: String,
    pub port: u16,
    pub db_path: String,
    #[serde(default)]
    pub asset_read_strict: bool,
}

pub async fn put_dashboard_preferences(
    State(state): State<AppState>,
    Json(body): Json<PutDashboardPreferences>,
) -> impl IntoResponse {
    let host = body.host.trim();
    if host.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "host required" })),
        )
            .into_response();
    }
    if body.port == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "invalid port" })),
        )
            .into_response();
    }
    let db_path = body.db_path.trim();
    if db_path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "db_path required" })),
        )
            .into_response();
    }

    let prefs = crate::schema::DashboardPreferences {
        host: host.into(),
        port: body.port,
        db_path: db_path.into(),
        asset_read_strict: body.asset_read_strict,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    match crate::preferences::save_preferences(&prefs) {
        Ok(path) => {
            let _ = crate::audit::record_audit(
                &state.db,
                crate::audit::AuditEventInput::low(
                    "dashboard_preferences_saved",
                    serde_json::json!({
                        "host": prefs.host,
                        "port": prefs.port,
                        "db_path": prefs.db_path,
                        "path": path.display().to_string(),
                    }),
                ),
            )
            .await;
            let active = active_preferences(&state);
            let restart_required = prefs.host != active.host
                || prefs.port != active.port
                || prefs.db_path != active.db_path;
            let view = crate::schema::DashboardPreferencesView {
                active,
                saved: Some(prefs.clone()),
                restart_command: crate::preferences::restart_command(
                    &prefs.host,
                    prefs.port,
                    std::path::Path::new(&prefs.db_path),
                ),
                preferences_path: path.display().to_string(),
                restart_required,
            };
            Json(json!({ "ok": true, "preferences": view })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn test_notification(
    State(state): State<AppState>,
    Json(body): Json<TestNotificationBody>,
) -> impl IntoResponse {
    match crate::notifications::send_test_notification(
        &state.db,
        body.project_id.as_deref(),
        &body.event_type,
    )
    .await
    {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct TestNotificationBody {
    pub event_type: String,
    pub project_id: Option<String>,
}

pub async fn patch_llm_config(
    State(state): State<AppState>,
    Json(body): Json<crate::config_patch::LlmConfigPatchBody>,
) -> impl IntoResponse {
    match crate::config_patch::patch_llm_config(&body) {
        Ok((path, cfg)) => {
            let _ = crate::audit::record_audit(
                &state.db,
                crate::audit::AuditEventInput {
                    project_id: None,
                    session_id: None,
                    action: "config_llm_updated".into(),
                    risk: "medium".into(),
                    detail: json!({ "config_path": path.display().to_string() }),
                },
            )
            .await;
            Json(json!({
                "ok": true,
                "config_path": path.display().to_string(),
                "provider": cfg.get("provider"),
                "model": cfg.get("model"),
                "model_fallback": cfg.get("runtime").and_then(|r| r.get("model_fallback")),
                "models": cfg.get("models"),
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

pub async fn get_llm_config() -> impl IntoResponse {
    let (_, cfg) = match crate::config_patch::read_config_value(None) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    let view = anycode_llm::RegistryView::from_config(&cfg);
    Json(json!({
        "config_present": view.config_present,
        "provider": view.provider,
        "model": view.model,
        "plan": view.plan,
        "base_url": view.base_url,
        "api_key": view.api_key,
        "provider_credentials": view.provider_credentials,
        "model_fallback": view.model_fallback,
        "models": view.models,
        "routing_agents": view.routing_agents,
        "registry": {
            "active": view.active,
            "items": view.items,
        }
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct TestLlmBody {
    pub capability: String,
}

pub async fn test_llm_config(Json(body): Json<TestLlmBody>) -> impl IntoResponse {
    let cap = match anycode_llm::ModelCapability::parse(&body.capability) {
        Some(c) => c,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "unknown capability" })),
            )
                .into_response();
        }
    };
    let (_, cfg) = match crate::config_patch::read_config_value(None) {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    match crate::llm_probe::LlmProbeService::from_config(&cfg)
        .probe(cap)
        .await
    {
        Ok(msg) => Json(json!({ "ok": true, "message": msg })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e })),
        )
            .into_response(),
    }
}

pub async fn get_db_operations(State(state): State<AppState>) -> impl IntoResponse {
    match crate::db_ops::db_operations(&state.db).await {
        Ok(ops) => Json(json!({ "operations": ops })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_api_tokens(State(state): State<AppState>) -> impl IntoResponse {
    match crate::tokens::list_tokens(&state.db).await {
        Ok(tokens) => Json(json!({ "tokens": tokens })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct CreateTokenBody {
    pub name: String,
    pub expires_days: Option<i64>,
}

pub async fn create_api_token(
    State(state): State<AppState>,
    Json(body): Json<CreateTokenBody>,
) -> impl IntoResponse {
    match crate::tokens::create_token(&state.db, &body.name, body.expires_days).await {
        Ok(created) => Json(json!({
            "token": created.record,
            "plaintext": created.plaintext,
            "warning": "Save this token now — it will not be shown again"
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn revoke_api_token(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
) -> impl IntoResponse {
    match crate::tokens::revoke_token(&state.db, &token_id).await {
        Ok(true) => Json(json!({ "ok": true })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "token not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_notification_policies(
    State(state): State<AppState>,
    Query(q): Query<NotificationQuery>,
) -> impl IntoResponse {
    match crate::notifications::list_notification_policies(&state.db, q.project_id.as_deref()).await
    {
        Ok(policies) => Json(json!({ "policies": policies })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct UpsertNotificationBody {
    pub project_id: Option<String>,
    pub event_type: String,
    pub channel: String,
    pub config: serde_json::Value,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub id: Option<String>,
}

pub async fn upsert_notification_policy(
    State(state): State<AppState>,
    Json(body): Json<UpsertNotificationBody>,
) -> impl IntoResponse {
    match crate::notifications::upsert_notification_policy(
        &state.db,
        body.project_id.as_deref(),
        &body.event_type,
        &body.channel,
        body.config,
        body.enabled,
        body.id.as_deref(),
    )
    .await
    {
        Ok(policy) => Json(json!({ "policy": policy })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_notification_policy(
    State(state): State<AppState>,
    Path(policy_id): Path<String>,
) -> impl IntoResponse {
    match crate::notifications::delete_notification_policy(&state.db, &policy_id).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct PolicyEnabledBody {
    pub enabled: bool,
}

pub async fn patch_notification_policy_enabled(
    State(state): State<AppState>,
    Path(policy_id): Path<String>,
    Json(body): Json<PolicyEnabledBody>,
) -> impl IntoResponse {
    match crate::notifications::set_notification_policy_enabled(&state.db, &policy_id, body.enabled)
        .await
    {
        Ok(policy) => Json(json!({ "policy": policy })).into_response(),
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

pub async fn list_connectors(
    State(state): State<AppState>,
    Query(q): Query<NotificationQuery>,
) -> impl IntoResponse {
    match crate::notifications::list_connectors(&state.db, q.project_id.as_deref()).await {
        Ok(connectors) => Json(json!({ "connectors": connectors })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct UpsertConnectorBody {
    pub project_id: Option<String>,
    pub source_type: String,
    pub name: String,
    pub config: serde_json::Value,
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub id: Option<String>,
}

pub async fn upsert_connector(
    State(state): State<AppState>,
    Json(body): Json<UpsertConnectorBody>,
) -> impl IntoResponse {
    match crate::notifications::upsert_connector(
        &state.db,
        body.project_id.as_deref(),
        &body.source_type,
        &body.name,
        body.config,
        body.enabled,
        body.id.as_deref(),
    )
    .await
    {
        Ok(connector) => Json(json!({ "connector": connector })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_connector(
    State(state): State<AppState>,
    Path(connector_id): Path<String>,
) -> impl IntoResponse {
    match crate::notifications::delete_connector(&state.db, &connector_id).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct ConnectorEnabledBody {
    pub enabled: bool,
}

pub async fn patch_connector_enabled(
    State(state): State<AppState>,
    Path(connector_id): Path<String>,
    Json(body): Json<ConnectorEnabledBody>,
) -> impl IntoResponse {
    match crate::notifications::set_connector_enabled(&state.db, &connector_id, body.enabled).await
    {
        Ok(connector) => Json(json!({ "connector": connector })).into_response(),
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

#[derive(Deserialize)]
pub struct MemoryRetentionQuery {
    #[serde(default = "default_retention_days")]
    pub older_than_days: i64,
}

fn default_retention_days() -> i64 {
    90
}

#[derive(Deserialize)]
pub struct MemoryRetentionApplyBody {
    #[serde(default = "default_retention_days")]
    pub older_than_days: i64,
    pub confirm: bool,
}

pub async fn get_memory_retention_preview(
    Query(q): Query<MemoryRetentionQuery>,
) -> impl IntoResponse {
    match crate::memory_ops::memory_retention_preview(q.older_than_days).await {
        Ok(v) => Json(v).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_memory_retention_apply(
    State(state): State<AppState>,
    Json(body): Json<MemoryRetentionApplyBody>,
) -> impl IntoResponse {
    if !body.confirm {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "confirm must be true" })),
        )
            .into_response();
    }
    match crate::memory_ops::memory_retention_apply(body.older_than_days).await {
        Ok(v) => {
            let _ = crate::audit::record_audit(
                &state.db,
                crate::audit::AuditEventInput::low(
                    "memory_retention_apply",
                    json!({
                        "older_than_days": body.older_than_days,
                        "summary": v.get("summary"),
                    }),
                ),
            )
            .await;
            Json(v).into_response()
        }
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
