pub mod auth;
mod handlers;
pub mod state;

pub fn spawn_cloud_a2a_heartbeat(state: AppState) {
    handlers::cloud_a2a::spawn_cloud_a2a_heartbeat(state);
}

use crate::api::state::AppState;
use axum::{
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode, Uri,
    },
    middleware,
    response::{Html, IntoResponse},
    routing::{any, delete, get, patch, post, put},
    Json, Router,
};
use serde_json::json;
use std::path::PathBuf;
use tower::ServiceBuilder;
use tower_http::{
    cors::{AllowOrigin, Any, CorsLayer},
    services::ServeDir,
    set_header::SetResponseHeaderLayer,
    trace::TraceLayer,
};

/// Hashed bundles under /assets/ — revalidate so localhost dev picks up new builds.
const UI_ASSET_CACHE: &str = "no-cache, must-revalidate";

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(handlers::health))
        .route("/cloud/session", get(handlers::get_cloud_session))
        .route("/cloud/link/start", post(handlers::post_cloud_link_start))
        .route("/cloud/link/poll", post(handlers::post_cloud_link_poll))
        .route(
            "/cloud/gateway-test",
            post(handlers::post_cloud_gateway_test),
        )
        .route("/cloud/sync-models", post(handlers::post_cloud_sync_models))
        .route("/cloud/unlink", post(handlers::post_cloud_unlink))
        .route(
            "/cloud/upstream/{*path}",
            any(handlers::proxy_cloud_upstream),
        )
        .route(
            "/cloud/a2a/heartbeat",
            post(handlers::post_cloud_a2a_heartbeat),
        )
        .route(
            "/cloud/a2a/team/peers",
            get(handlers::get_cloud_a2a_team_peers),
        )
        .route(
            "/cloud/a2a/handoff/request",
            post(handlers::post_cloud_a2a_handoff_request),
        )
        .route(
            "/cloud/a2a/handoff/incoming",
            get(handlers::get_cloud_a2a_handoff_incoming),
        )
        .route(
            "/cloud/a2a/handoff/outgoing",
            get(handlers::get_cloud_a2a_handoff_outgoing),
        )
        .route(
            "/cloud/a2a/handoff/{handoff_id}/approve",
            post(handlers::post_cloud_a2a_handoff_approve),
        )
        .route(
            "/cloud/a2a/handoff/{handoff_id}/reject",
            post(handlers::post_cloud_a2a_handoff_reject),
        )
        .route("/local-models", get(handlers::list_local_models))
        .route(
            "/local-models/{model_id}",
            get(handlers::get_local_model).delete(handlers::delete_local_model),
        )
        .route(
            "/local-models/{model_id}/download",
            post(handlers::post_local_model_download),
        )
        .route(
            "/local-models/{model_id}/download/cancel",
            post(handlers::post_local_model_cancel_download),
        )
        .route(
            "/local-models/{model_id}/start",
            post(handlers::post_local_model_start),
        )
        .route(
            "/local-models/{model_id}/stop",
            post(handlers::post_local_model_stop),
        )
        .route("/local-llm/status", get(handlers::get_managed_local_status))
        .route(
            "/local-llm/download",
            post(handlers::post_managed_local_download),
        )
        .route(
            "/local-llm/download/cancel",
            post(handlers::post_managed_local_cancel_download),
        )
        .route("/local-llm/start", post(handlers::post_managed_local_start))
        .route("/local-llm/stop", post(handlers::post_managed_local_stop))
        .route(
            "/local-llm/model",
            delete(handlers::delete_managed_local_model),
        )
        .route("/auth/me", get(handlers::get_auth_me))
        .route("/auth/login", post(handlers::post_auth_login))
        .route("/auth/logout", post(handlers::post_auth_logout))
        .route(
            "/auth/desktop-bootstrap",
            get(handlers::get_desktop_bootstrap),
        )
        .route("/bootstrap", get(handlers::get_bootstrap))
        .route("/setup/status", get(handlers::get_setup_status))
        .route("/setup/quick-auth", get(handlers::get_setup_quick_auth))
        .route(
            "/setup/workspace/ensure",
            post(handlers::post_setup_workspace_ensure),
        )
        .route("/setup/memory", patch(handlers::patch_setup_memory))
        .route("/setup/complete", post(handlers::post_setup_complete))
        .route("/overview", get(handlers::get_overview))
        .route("/overview/briefing", post(handlers::post_overview_briefing))
        .route("/reports/recent", get(handlers::list_recent_reports))
        .route("/metrics/readiness", get(handlers::get_delivery_readiness))
        .route("/metrics/timeline", get(handlers::get_timeline_metrics))
        .route("/metrics/usage", get(handlers::get_usage_metrics))
        .route("/metrics/usage/export", get(handlers::export_usage_metrics))
        .route(
            "/metrics/kpi/saved-hours",
            get(handlers::get_saved_hours_kpi),
        )
        .route("/security/activity", get(handlers::get_security_activity))
        .route("/governance/tools", get(handlers::get_tool_governance))
        .route(
            "/security/approvals/pending",
            get(handlers::list_pending_approvals),
        )
        .route(
            "/security/approvals/summary",
            get(handlers::get_approval_summary),
        )
        .route(
            "/security/approvals/{approval_id}/respond",
            post(handlers::respond_to_approval),
        )
        .route(
            "/security/questions/pending",
            get(handlers::list_pending_questions),
        )
        .route(
            "/security/questions/{question_id}/respond",
            post(handlers::respond_to_question),
        )
        .route(
            "/notifications/recent",
            get(handlers::list_recent_notifications),
        )
        .route("/search", get(handlers::search_workbench))
        .route(
            "/projects/{project_id}/skills",
            get(handlers::list_project_skills),
        )
        .route("/artifacts", get(handlers::list_artifacts))
        .route("/assets", get(handlers::list_assets))
        .route("/assets/{asset_id}", get(handlers::get_asset_detail))
        .route(
            "/assets/{asset_id}/mark-reusable",
            post(handlers::mark_asset_reusable),
        )
        .route("/assets/{asset_id}/archive", post(handlers::archive_asset))
        .route(
            "/assets/{asset_id}/promote-skill-draft",
            post(handlers::promote_skill_draft),
        )
        .route(
            "/assets/{asset_id}/promote-workflow-draft",
            post(handlers::promote_workflow_draft),
        )
        .route(
            "/projects/{project_id}/scan-workflows",
            post(handlers::scan_project_workflows),
        )
        .route(
            "/artifacts/{artifact_id}",
            get(handlers::get_artifact_detail),
        )
        .route(
            "/skills/{skill_id}",
            get(handlers::get_skill_detail).delete(handlers::uninstall_skill),
        )
        .route("/sessions/running", get(handlers::list_running_sessions))
        .route(
            "/skills",
            get(handlers::list_skills).post(handlers::rescan_skills),
        )
        .route(
            "/projects/{project_id}/index-assets",
            post(handlers::index_project_assets),
        )
        .route(
            "/projects/{project_id}/metrics",
            get(handlers::get_project_metrics),
        )
        .route(
            "/projects/{project_id}/usage",
            get(handlers::get_project_usage),
        )
        .route(
            "/projects/{project_id}/gates/presets",
            get(handlers::list_gate_presets),
        )
        .route(
            "/projects/{project_id}/gates/execute",
            post(handlers::execute_project_gate),
        )
        .route(
            "/projects/{project_id}/gates/execute/stream",
            post(handlers::execute_project_gate_stream),
        )
        .route(
            "/projects/{project_id}/conversations/start",
            post(handlers::start_project_conversation),
        )
        .route(
            "/projects/{project_id}/runs/trigger",
            post(handlers::trigger_project_run),
        )
        .route(
            "/projects/{project_id}/runs/triggers",
            get(handlers::list_project_triggers),
        )
        .route(
            "/projects/{project_id}/automation-policies",
            get(handlers::list_automation_policies).post(handlers::upsert_automation_policy),
        )
        .route(
            "/projects/{project_id}/automation-policies/{policy_id}",
            delete(handlers::delete_automation_policy),
        )
        .route(
            "/projects/{project_id}/skills/{skill_id}",
            put(handlers::set_project_skill),
        )
        .route(
            "/skills/{skill_id}/all-projects",
            post(handlers::set_skill_all_projects),
        )
        .route("/cron/runs", get(handlers::list_cron_runs))
        .route(
            "/cron/jobs",
            get(handlers::list_cron_jobs).post(handlers::create_cron_job),
        )
        .route(
            "/cron/jobs/{job_id}",
            delete(handlers::delete_cron_job).patch(handlers::patch_cron_job),
        )
        .route("/cron/parse-schedule", post(handlers::parse_cron_schedule))
        .route("/cron/retry", post(handlers::retry_cron_job))
        .route("/cron/templates", get(handlers::list_automation_templates))
        .route(
            "/orchestration/tasks",
            get(handlers::list_orchestration_tasks),
        )
        .route("/skills/market", get(handlers::list_skill_market))
        .route(
            "/skills/market/install",
            post(handlers::install_market_skill),
        )
        .route("/skills/import", post(handlers::import_skill))
        .route(
            "/projects/{project_id}/knowledge",
            get(handlers::get_project_knowledge).put(handlers::put_project_knowledge),
        )
        .route(
            "/projects/{project_id}/knowledge/reindex",
            post(handlers::reindex_project_knowledge),
        )
        .route(
            "/projects/{project_id}/knowledge/search",
            get(handlers::search_project_knowledge),
        )
        .route(
            "/projects/{project_id}/knowledge/stats",
            get(handlers::get_project_knowledge_stats),
        )
        .route(
            "/projects/{project_id}/fs/list",
            get(handlers::list_project_fs),
        )
        .route(
            "/projects/{project_id}/fs/read",
            get(handlers::read_project_fs),
        )
        .route(
            "/projects/{project_id}/fs/raw",
            get(handlers::raw_project_fs),
        )
        .route(
            "/projects/{project_id}/fs/stat",
            get(handlers::stat_project_fs),
        )
        .route(
            "/projects/{project_id}/terminal/ws",
            get(handlers::project_terminal_ws),
        )
        .route(
            "/workbench/browser/status",
            get(handlers::get_workbench_browser_status),
        )
        .route(
            "/workbench/browser/sessions",
            post(handlers::create_browser_session),
        )
        .route(
            "/workbench/browser/sessions/{session_id}/navigate",
            post(handlers::navigate_browser_session),
        )
        .route(
            "/workbench/browser/sessions/{session_id}/state",
            get(handlers::browser_session_state),
        )
        .route(
            "/workbench/browser/sessions/{session_id}/screenshot",
            get(handlers::browser_session_screenshot),
        )
        .route(
            "/workbench/browser/sessions/{session_id}/stream",
            get(handlers::browser_session_stream),
        )
        .route(
            "/workbench/browser/sessions/{session_id}/lock",
            post(handlers::browser_session_lock),
        )
        .route(
            "/workbench/browser/sessions/{session_id}",
            delete(handlers::delete_browser_session),
        )
        .route("/skills/suggestions", get(handlers::get_skill_suggestions))
        .route(
            "/skills/install-starter",
            post(handlers::install_starter_skills),
        )
        .route("/agents/stats", get(handlers::list_agent_stats))
        .route("/agents/profiles", get(handlers::list_agent_profiles))
        .route(
            "/agents/profiles/{id}",
            get(handlers::get_agent_profile)
                .put(handlers::put_agent_profile)
                .delete(handlers::delete_agent_profile),
        )
        .route(
            "/agents/profiles/{id}/effective",
            get(handlers::get_agent_profile_effective),
        )
        .route("/events/stream", get(handlers::global_events_stream))
        .route("/events/{event_id}", get(handlers::get_event))
        .route("/events", get(handlers::list_recent_events))
        .route("/project-templates", get(handlers::list_project_templates))
        .route(
            "/projects",
            get(handlers::list_projects).post(handlers::upsert_project),
        )
        .route("/projects/scan", post(handlers::scan_projects))
        .route(
            "/projects/{project_id}",
            get(handlers::get_project).patch(handlers::patch_project),
        )
        .route(
            "/projects/{project_id}/status",
            axum::routing::patch(handlers::patch_project_status),
        )
        .route(
            "/projects/{project_id}/view-prefs",
            get(handlers::get_project_view_prefs).put(handlers::put_project_view_prefs),
        )
        .route(
            "/projects/{project_id}/stats",
            get(handlers::get_project_stats),
        )
        .route(
            "/projects/{project_id}/sessions",
            get(handlers::list_project_sessions),
        )
        .route(
            "/projects/{project_id}/events/stream",
            get(handlers::project_events_stream),
        )
        .route(
            "/projects/{project_id}/event-types",
            get(handlers::list_project_event_types),
        )
        .route(
            "/projects/{project_id}/events",
            get(handlers::list_project_events).post(handlers::insert_project_event),
        )
        .route(
            "/projects/{project_id}/events/publish",
            post(handlers::publish_project_event),
        )
        .route(
            "/projects/{project_id}/gates",
            get(handlers::list_project_gates),
        )
        .route(
            "/projects/{project_id}/artifacts",
            get(handlers::list_project_artifacts),
        )
        .route(
            "/projects/{project_id}/reindex",
            post(handlers::reindex_project),
        )
        .route(
            "/projects/{project_id}/report",
            get(handlers::get_project_report),
        )
        .route(
            "/projects/{project_id}/data-health",
            get(handlers::get_project_data_health),
        )
        .route(
            "/sessions/{session_id}/events/stream",
            get(handlers::session_events_stream),
        )
        .route(
            "/sessions/{session_id}/event-types",
            get(handlers::list_session_event_types),
        )
        .route(
            "/sessions",
            get(handlers::list_all_sessions).post(handlers::create_session),
        )
        .route("/sessions/facets", get(handlers::list_session_facets))
        .route(
            "/sessions/{session_id}",
            get(handlers::get_session).patch(handlers::patch_session),
        )
        .route(
            "/sessions/{session_id}/message",
            axum::routing::post(handlers::send_session_message),
        )
        .route(
            "/sessions/{session_id}/message-queue",
            get(handlers::list_session_message_queue),
        )
        .route(
            "/sessions/{session_id}/message-queue/{queue_id}",
            axum::routing::delete(handlers::cancel_session_message_queue_item),
        )
        .route(
            "/sessions/{session_id}/cancel",
            axum::routing::post(handlers::cancel_session),
        )
        .route(
            "/sessions/{session_id}/acknowledge-block",
            axum::routing::post(handlers::acknowledge_session_block),
        )
        .route(
            "/sessions/{session_id}/auto-approve",
            get(handlers::get_session_auto_approve).post(handlers::set_session_auto_approve),
        )
        .route(
            "/sessions/{session_id}/usage",
            get(handlers::get_session_usage),
        )
        .route(
            "/sessions/{session_id}/replay",
            get(handlers::get_session_replay),
        )
        .route(
            "/sessions/{session_id}/trace",
            get(handlers::get_session_trace),
        )
        .route(
            "/sessions/{session_id}/transcript",
            get(handlers::get_session_transcript),
        )
        .route(
            "/sessions/{session_id}/execution-log",
            get(handlers::get_session_execution_log),
        )
        .route(
            "/sessions/{session_id}/report",
            get(handlers::get_session_report),
        )
        .route(
            "/sessions/{session_id}/events",
            get(handlers::list_session_events),
        )
        .route(
            "/sessions/{session_id}/gates",
            get(handlers::list_session_gates),
        )
        .route(
            "/sessions/{session_id}/artifacts",
            get(handlers::list_session_artifacts),
        )
        .route(
            "/sessions/{session_id}/scan-artifacts",
            post(handlers::scan_session_artifacts),
        )
        .route(
            "/sessions/{session_id}/background-tasks",
            get(handlers::get_session_background_tasks),
        )
        .route("/media/status", get(handlers::get_media_status))
        .route("/media/transcribe", post(handlers::transcribe_audio))
        .route("/media/ocr", post(handlers::ocr_images))
        .route("/media/tts", post(handlers::synthesize_speech))
        .route("/settings/services", get(handlers::list_services))
        .route(
            "/settings/service-status",
            get(handlers::get_service_status),
        )
        .route("/settings/doctor", get(handlers::get_doctor))
        .route("/settings/runtime", get(handlers::get_runtime_settings))
        .route("/settings/model-catalog", get(handlers::get_model_catalog))
        .route(
            "/settings/model-catalog/refresh",
            post(handlers::refresh_model_catalog),
        )
        .route(
            "/settings/models",
            get(handlers::get_models_registry).put(handlers::put_models_registry),
        )
        .route(
            "/settings/models/{model_id}/enable",
            post(handlers::enable_model),
        )
        .route(
            "/settings/models/{model_id}/test",
            post(handlers::test_model),
        )
        .route(
            "/settings/llm",
            get(handlers::get_llm_config)
                .put(handlers::patch_llm_config)
                .post(handlers::test_llm_config),
        )
        .route(
            "/settings/preferences",
            get(handlers::get_dashboard_preferences).put(handlers::put_dashboard_preferences),
        )
        .route(
            "/settings/gate-prefs",
            get(handlers::get_gate_preferences).put(handlers::put_gate_preferences),
        )
        .route("/settings/database", get(handlers::database_settings))
        .route(
            "/settings/database/backup",
            post(handlers::post_database_backup),
        )
        .route("/settings/db-operations", get(handlers::get_db_operations))
        .route(
            "/settings/memory/retention",
            get(handlers::get_memory_retention_preview).post(handlers::post_memory_retention_apply),
        )
        .route("/settings/memory/center", get(handlers::get_memory_center))
        .route("/settings/memory/dream", post(handlers::post_memory_dream))
        .route("/settings/policies", get(handlers::get_policy_summary))
        .route("/settings/data-health", get(handlers::get_data_health))
        .route(
            "/settings/tokens",
            get(handlers::list_api_tokens).post(handlers::create_api_token),
        )
        .route(
            "/settings/tokens/{token_id}/revoke",
            post(handlers::revoke_api_token),
        )
        .route(
            "/settings/notifications",
            get(handlers::list_notification_policies).post(handlers::upsert_notification_policy),
        )
        .route(
            "/settings/notifications/{policy_id}",
            axum::routing::delete(handlers::delete_notification_policy),
        )
        .route(
            "/settings/notifications/{policy_id}/enabled",
            axum::routing::patch(handlers::patch_notification_policy_enabled),
        )
        .route(
            "/settings/notifications/test",
            post(handlers::test_notification),
        )
        .route(
            "/settings/browser-connector",
            get(handlers::get_browser_connector).put(handlers::put_browser_connector),
        )
        .route(
            "/settings/agent-limits",
            get(handlers::get_agent_limits).put(handlers::put_agent_limits),
        )
        .route(
            "/settings/mcp-servers",
            get(handlers::get_mcp_servers).put(handlers::put_mcp_servers),
        )
        .route(
            "/settings/prompt-preview",
            get(handlers::get_prompt_preview),
        )
        .route(
            "/settings/prompt-settings",
            put(handlers::put_prompt_settings),
        )
        .route(
            "/settings/connectors",
            get(handlers::list_connectors).post(handlers::upsert_connector),
        )
        .route(
            "/settings/connectors/{connector_id}",
            axum::routing::delete(handlers::delete_connector),
        )
        .route(
            "/settings/connectors/{connector_id}/enabled",
            axum::routing::patch(handlers::patch_connector_enabled),
        )
        .route(
            "/settings/connectors/{connector_id}/github/issues",
            get(handlers::get_connector_github_issues),
        )
        .route(
            "/settings/connectors/{connector_id}/linear/issues",
            get(handlers::get_connector_linear_issues),
        )
        .route("/lan/peers", get(handlers::get_lan_peers))
        .route(
            "/lan/settings",
            get(handlers::get_lan_settings).patch(handlers::patch_lan_settings),
        )
        .route(
            "/lan/handoff/request",
            post(handlers::post_lan_handoff_request),
        )
        .route(
            "/lan/handoff/incoming",
            get(handlers::get_lan_handoff_incoming),
        )
        .route(
            "/lan/handoff/outgoing",
            get(handlers::get_lan_handoff_outgoing),
        )
        .route(
            "/lan/handoff/{handoff_id}/approve",
            post(handlers::post_lan_handoff_approve),
        )
        .route(
            "/lan/handoff/{handoff_id}/reject",
            post(handlers::post_lan_handoff_reject),
        )
        .route("/audit/events", get(handlers::list_audit_events))
        .route("/plugins", get(handlers::list_plugins))
        .route(
            "/plugins/{plugin_id}",
            axum::routing::put(handlers::put_plugin_enabled),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::mutate_origin_guard,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
        .with_state(state.clone());

    let mut app = Router::new().nest("/api", api);

    if !state.serve_ui {
        app = app.route(
            "/",
            get(|| async {
                Json(json!({
                    "error": "api_only",
                    "hint": "Open anyCode.app for the Workbench UI"
                }))
            }),
        );
    } else if let Some(dir) = &state.static_dir {
        let index = dir.join("index.html");
        if index.is_file() {
            let assets = dir.join("assets");
            if assets.is_dir() {
                // SPA route `/assets` (artifacts page) must not collide with Vite bundle mount.
                let index_for_assets_page = index.clone();
                app = app.route(
                    "/assets",
                    get(move || serve_spa_index(index_for_assets_page.clone())),
                );
                let asset_service = ServiceBuilder::new()
                    .layer(SetResponseHeaderLayer::overriding(
                        CACHE_CONTROL,
                        HeaderValue::from_static(UI_ASSET_CACHE),
                    ))
                    .service(ServeDir::new(assets));
                app = app.nest_service("/assets/", asset_service);
            }
            let static_root = dir.clone();
            let index_for_fallback = index.clone();
            app = app.fallback(get(move |uri: Uri| async move {
                spa_fallback(uri, static_root.clone(), index_for_fallback.clone()).await
            }));
        }
    } else if crate::embedded_ui::available() {
        app = app.fallback(get(crate::embedded_ui::fallback));
    }

    app.layer(TraceLayer::new_for_http()).layer(cors_layer())
}

fn cors_layer() -> CorsLayer {
    let list: Vec<axum::http::HeaderValue> = auth::ALLOWED_BROWSER_ORIGINS
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(list))
        .allow_methods(Any)
        .allow_headers(Any)
}

async fn spa_fallback(uri: Uri, static_root: PathBuf, index: PathBuf) -> axum::response::Response {
    if uri.path().starts_with("/api/") {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "API route not found" })),
        )
            .into_response();
    }
    let rel = uri.path().trim_start_matches('/');
    if !rel.is_empty() && !rel.contains("..") {
        let file = static_root.join(rel);
        if file.is_file() {
            return serve_static_file(file).await.into_response();
        }
    }
    serve_spa_index(index).await.into_response()
}

async fn serve_static_file(path: PathBuf) -> impl IntoResponse {
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mut headers = HeaderMap::new();
            headers.insert(CACHE_CONTROL, HeaderValue::from_static(UI_ASSET_CACHE));
            if let Some(ct) = mime_for_path(&path) {
                headers.insert(CONTENT_TYPE, HeaderValue::from_static(ct));
            }
            (headers, bytes).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

fn mime_for_path(path: &PathBuf) -> Option<&'static str> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| match ext {
            "png" => "image/png",
            "ico" => "image/x-icon",
            "svg" => "image/svg+xml",
            "css" => "text/css; charset=utf-8",
            "js" => "text/javascript; charset=utf-8",
            "json" => "application/json; charset=utf-8",
            "woff2" => "font/woff2",
            _ => "application/octet-stream",
        })
}

async fn serve_spa_index(index: PathBuf) -> impl IntoResponse {
    match tokio::fs::read_to_string(index).await {
        Ok(html) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                CACHE_CONTROL,
                HeaderValue::from_static("no-store, no-cache, must-revalidate"),
            );
            headers.insert(
                CONTENT_TYPE,
                HeaderValue::from_static("text/html; charset=utf-8"),
            );
            (headers, Html(html)).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
