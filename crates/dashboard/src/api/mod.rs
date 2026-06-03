mod auth;
mod handlers;
pub mod state;

use crate::api::state::AppState;
use axum::{
    http::{
        header::{CACHE_CONTROL, CONTENT_TYPE},
        HeaderMap, HeaderValue, StatusCode,
    },
    middleware,
    response::{Html, IntoResponse},
    routing::{delete, get, post, put},
    Router,
};
use std::path::PathBuf;
use tower_http::{
    cors::{Any, CorsLayer},
    services::ServeDir,
    trace::TraceLayer,
};

pub fn router(state: AppState) -> Router {
    let api = Router::new()
        .route("/health", get(handlers::health))
        .route("/auth/me", get(handlers::get_auth_me))
        .route("/auth/login", post(handlers::post_auth_login))
        .route("/auth/logout", post(handlers::post_auth_logout))
        .route("/bootstrap", get(handlers::get_bootstrap))
        .route("/overview", get(handlers::get_overview))
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
            "/notifications/recent",
            get(handlers::list_recent_notifications),
        )
        .route("/search", get(handlers::search_workbench))
        .route(
            "/projects/{project_id}/skills",
            get(handlers::list_project_skills),
        )
        .route("/artifacts", get(handlers::list_artifacts))
        .route(
            "/artifacts/{artifact_id}",
            get(handlers::get_artifact_detail),
        )
        .route("/skills/{skill_id}", get(handlers::get_skill_detail))
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
        .route("/cron/parse-schedule", post(handlers::parse_cron_schedule))
        .route("/cron/retry", post(handlers::retry_cron_job))
        .route("/cron/templates", get(handlers::list_automation_templates))
        .route(
            "/orchestration/tasks",
            get(handlers::list_orchestration_tasks),
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
        .route(
            "/projects",
            get(handlers::list_projects).post(handlers::upsert_project),
        )
        .route("/projects/scan", post(handlers::scan_projects))
        .route("/projects/{project_id}", get(handlers::get_project))
        .route(
            "/projects/{project_id}/status",
            axum::routing::patch(handlers::patch_project_status),
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
        .route("/sessions/{session_id}", get(handlers::get_session))
        .route(
            "/sessions/{session_id}/message",
            axum::routing::post(handlers::send_session_message),
        )
        .route(
            "/sessions/{session_id}/cancel",
            axum::routing::post(handlers::cancel_session),
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
            "/sessions/{session_id}/background-tasks",
            get(handlers::get_session_background_tasks),
        )
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
        .route("/settings/database", get(handlers::database_settings))
        .route("/settings/db-operations", get(handlers::get_db_operations))
        .route(
            "/settings/memory/retention",
            get(handlers::get_memory_retention_preview).post(handlers::post_memory_retention_apply),
        )
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
        .route("/audit/events", get(handlers::list_audit_events))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
        .with_state(state.clone());

    let mut app = Router::new().nest("/api", api);

    if let Some(dir) = &state.static_dir {
        let index = dir.join("index.html");
        if index.is_file() {
            let assets = dir.join("assets");
            if assets.is_dir() {
                app = app.route_service("/assets/{*path}", ServeDir::new(dir.clone()));
            }
            app = app.fallback(get(move || serve_spa_index(index.clone())));
        }
    } else if crate::embedded_ui::available() {
        app = app.fallback(get(crate::embedded_ui::fallback));
    }

    app.layer(TraceLayer::new_for_http()).layer(
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any),
    )
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
