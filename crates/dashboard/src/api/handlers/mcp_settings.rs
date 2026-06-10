use super::*;
use crate::config_patch::{read_config_root, write_config_root};
use crate::mcp_config;

#[derive(Deserialize)]
pub struct McpServersBody {
    pub servers: Vec<serde_json::Value>,
}

pub async fn get_mcp_servers() -> impl IntoResponse {
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
    let servers = mcp_config::read_mcp_servers(&cfg);
    Json(json!({
        "servers": mcp_config::redact_mcp_servers(&servers),
    }))
    .into_response()
}

pub async fn put_mcp_servers(Json(body): Json<McpServersBody>) -> impl IntoResponse {
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
    mcp_config::set_mcp_servers(&mut cfg, body.servers.clone());
    match write_config_root(&cfg) {
        Ok(path) => Json(json!({
            "ok": true,
            "servers": mcp_config::redact_mcp_servers(&body.servers),
            "config_path": path.display().to_string(),
            "restart_hint": "Start a new conversation or restart the app for MCP servers to attach."
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
