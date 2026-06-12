//! Dashboard setup wizard API handlers.

use super::*;
use anycode_setup::{
    apply_memory_preset, ensure_layout, fetch_wechat_qr, load_setup_status,
    memory_preset_from_label, poll_wechat_qr_status, quick_auth_presets,
};
use axum::Json;
use serde::Deserialize;

pub async fn get_setup_status(State(state): State<AppState>) -> impl IntoResponse {
    let prefs = crate::preferences::load_preferences();
    let setup_at = prefs.as_ref().and_then(|p| p.setup_completed_at.clone());
    let stats = state.db.overview_stats().await.ok();
    let projects_count = stats.map(|s| s.projects_count).unwrap_or(0);
    let status = load_setup_status(setup_at.as_deref(), projects_count);
    Json(json!({ "setup": status })).into_response()
}

pub async fn get_setup_quick_auth() -> impl IntoResponse {
    Json(json!({ "presets": quick_auth_presets() })).into_response()
}

pub async fn post_setup_workspace_ensure() -> impl IntoResponse {
    match ensure_layout() {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct SetupMemoryBody {
    pub preset: String,
    #[serde(default)]
    pub embedding_base_url: Option<String>,
    #[serde(default)]
    pub embedding_model: Option<String>,
}

pub async fn patch_setup_memory(Json(body): Json<SetupMemoryBody>) -> impl IntoResponse {
    let preset = if body.preset == "pipeline_http" {
        let (Some(url), Some(model)) = (
            body.embedding_base_url.as_deref(),
            body.embedding_model.as_deref(),
        ) else {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "ok": false,
                    "error": "pipeline_http requires embedding_base_url and embedding_model"
                })),
            )
                .into_response();
        };
        if url.trim().is_empty() || model.trim().is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "ok": false,
                    "error": "pipeline_http requires non-empty embedding_base_url and embedding_model"
                })),
            )
                .into_response();
        }
        memory_preset_from_label(&format!("pipeline_http:{}|{}", url.trim(), model.trim()))
    } else {
        memory_preset_from_label(&body.preset)
    };

    let preset = match preset {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "ok": false, "error": e.to_string() })),
            )
                .into_response();
        }
    };

    match crate::config_patch::read_config_root() {
        Ok((_path, mut cfg)) => {
            apply_memory_preset(&mut cfg, preset);
            match crate::config_patch::write_config_root(&cfg) {
                Ok(path) => Json(json!({ "ok": true, "config_path": path.display().to_string() }))
                    .into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "ok": false, "error": e.to_string() })),
                )
                    .into_response(),
            }
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_setup_channels_wechat_qr() -> impl IntoResponse {
    match fetch_wechat_qr().await {
        Ok(qr) => Json(json!({ "ok": true, "qr": qr })).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct WechatPollQuery {
    pub qrcode_id: String,
}

pub async fn get_setup_channels_wechat_status(
    Query(q): Query<WechatPollQuery>,
) -> impl IntoResponse {
    if q.qrcode_id.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": "qrcode_id required" })),
        )
            .into_response();
    }
    match poll_wechat_qr_status(q.qrcode_id.trim()).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Deserialize, Default)]
pub struct SetupCompleteBody {
    #[serde(default)]
    pub scan_projects: bool,
}

pub async fn post_setup_complete(
    State(state): State<AppState>,
    Json(body): Json<SetupCompleteBody>,
) -> impl IntoResponse {
    let active = crate::api::handlers::settings::active_preferences(&state);
    let mut prefs = crate::preferences::load_preferences().unwrap_or(active);
    prefs.setup_completed_at = Some(chrono::Utc::now().to_rfc3339());
    prefs.updated_at = prefs.setup_completed_at.clone().unwrap_or_default();

    if body.scan_projects {
        let _ = state.db.sync_workspace_paths(&state.workspace_paths).await;
    }

    match crate::preferences::save_preferences(&prefs) {
        Ok(path) => Json(json!({
            "ok": true,
            "preferences_path": path.display().to_string(),
            "setup_completed_at": prefs.setup_completed_at,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}
