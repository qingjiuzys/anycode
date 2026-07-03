//! Plugin registry API for dashboard settings.

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::json;

use crate::api::state::AppState;
use crate::service_governance::is_loopback_host;

pub async fn list_plugins(State(state): State<AppState>) -> impl IntoResponse {
    if !is_loopback_host(&state.host) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "plugins_loopback_only" })),
        )
            .into_response();
    }
    let workspace = state.workspace_paths.first().map(std::path::Path::new);
    let plugins = anycode_agent::plugins::load_plugins(workspace);
    let public: Vec<_> = plugins
        .into_iter()
        .map(|p| {
            json!({
                "id": p.id,
                "name": p.name,
                "version": p.version,
                "enabled": p.enabled,
                "priority": p.priority,
                "tools": p.tools,
                "overlay_preview": p.system_prompt_overlay.as_deref().map(|s| {
                    let t = s.trim();
                    if t.len() > 240 { format!("{}…", &t[..240]) } else { t.to_string() }
                }),
            })
        })
        .collect();
    Json(json!({ "plugins": public })).into_response()
}

#[derive(Deserialize)]
pub struct PutPluginBody {
    pub enabled: bool,
}

pub async fn put_plugin_enabled(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(body): Json<PutPluginBody>,
) -> impl IntoResponse {
    if !is_loopback_host(&state.host) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "plugins_loopback_only" })),
        )
            .into_response();
    }
    match anycode_agent::plugins::set_plugin_enabled(&id, body.enabled) {
        Ok(()) => Json(json!({ "ok": true, "id": id, "enabled": body.enabled })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
