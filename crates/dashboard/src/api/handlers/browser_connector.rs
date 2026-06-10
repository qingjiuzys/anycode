use super::*;
use crate::browser_connector;
use crate::config_patch::{read_config_root, write_config_root};

#[derive(Deserialize)]
pub struct BrowserConnectorBody {
    pub enabled: bool,
}

pub async fn get_browser_connector() -> impl IntoResponse {
    let (_, cfg) = match read_config_root() {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    let enabled = browser_connector::read_browser_enabled(&cfg);
    let mut status = browser_connector::browser_connector_status();
    if let Some(obj) = status.as_object_mut() {
        obj.insert("enabled".into(), json!(enabled));
    }
    Json(status).into_response()
}

pub async fn put_browser_connector(Json(body): Json<BrowserConnectorBody>) -> impl IntoResponse {
    let (_, mut cfg) = match read_config_root() {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    if body.enabled && !browser_connector::resolve_browser_mcp_bundle_root().is_some() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "browser_mcp_bundle_missing",
                "message": "Bundled browser MCP not found. Reinstall the desktop app or set ANYCODE_BROWSER_MCP_ROOT."
            })),
        )
            .into_response();
    }
    browser_connector::set_browser_enabled(&mut cfg, body.enabled);
    match write_config_root(&cfg) {
        Ok(path) => Json(json!({
            "ok": true,
            "enabled": body.enabled,
            "config_path": path.display().to_string(),
            "restart_hint": "Start a new conversation or restart the desktop app for MCP tools to attach."
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
