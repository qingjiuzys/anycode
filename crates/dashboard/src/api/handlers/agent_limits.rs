use super::*;
use crate::config_patch::{read_config_root, write_config_root};
use anycode_core::{
    resolve_agent_loop_limits, AgentLoopLimits, DEFAULT_MAX_AGENT_TURNS, DEFAULT_MAX_TOOL_CALLS,
    MAX_AGENT_TURNS_CLAMP, MAX_TOOL_CALLS_CLAMP,
};
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct AgentLimitsBody {
    pub max_agent_turns: usize,
    pub max_tool_calls: usize,
}

fn read_runtime_usize(cfg: &Value, key: &str) -> Option<usize> {
    cfg.get("runtime")
        .and_then(|r| r.get(key))
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
}

fn write_runtime_limits(cfg: &mut Value, limits: AgentLoopLimits) {
    let Some(root) = cfg.as_object_mut() else {
        return;
    };
    let runtime = root
        .entry("runtime")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("runtime object");
    runtime.insert("max_agent_turns".into(), json!(limits.max_agent_turns));
    runtime.insert("max_tool_calls".into(), json!(limits.max_tool_calls));
}

fn limits_payload(cfg: &Value, config_path: &std::path::Path) -> Value {
    let configured_turns = read_runtime_usize(cfg, "max_agent_turns");
    let configured_tools = read_runtime_usize(cfg, "max_tool_calls");
    let resolved = resolve_agent_loop_limits(configured_turns, configured_tools);
    json!({
        "max_agent_turns": resolved.max_agent_turns,
        "max_tool_calls": resolved.max_tool_calls,
        "configured_max_agent_turns": configured_turns,
        "configured_max_tool_calls": configured_tools,
        "defaults": {
            "max_agent_turns": DEFAULT_MAX_AGENT_TURNS,
            "max_tool_calls": DEFAULT_MAX_TOOL_CALLS,
        },
        "limits": {
            "max_agent_turns_min": 1,
            "max_agent_turns_max": MAX_AGENT_TURNS_CLAMP,
            "max_tool_calls_min": 1,
            "max_tool_calls_max": MAX_TOOL_CALLS_CLAMP,
        },
        "config_path": config_path.display().to_string(),
        "restart_hint": "Start a new conversation or restart the CLI for updated limits to apply.",
    })
}

pub async fn get_agent_limits() -> impl IntoResponse {
    let (path, cfg) = match read_config_root() {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    Json(limits_payload(&cfg, &path)).into_response()
}

pub async fn put_agent_limits(Json(body): Json<AgentLimitsBody>) -> impl IntoResponse {
    if body.max_agent_turns == 0 || body.max_tool_calls == 0 {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "invalid_limits",
                "message": "max_agent_turns and max_tool_calls must be at least 1",
            })),
        )
            .into_response();
    }
    let limits = AgentLoopLimits::clamped(body.max_agent_turns, body.max_tool_calls);
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
    write_runtime_limits(&mut cfg, limits);
    match write_config_root(&cfg) {
        Ok(path) => Json(json!({
            "ok": true,
            "max_agent_turns": limits.max_agent_turns,
            "max_tool_calls": limits.max_tool_calls,
            "config_path": path.display().to_string(),
            "restart_hint": "Start a new conversation or restart the CLI for updated limits to apply.",
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_runtime_limits_roundtrip() {
        let mut cfg = json!({});
        let limits = AgentLoopLimits::clamped(12, 48);
        write_runtime_limits(&mut cfg, limits);
        assert_eq!(read_runtime_usize(&cfg, "max_agent_turns"), Some(12));
        assert_eq!(read_runtime_usize(&cfg, "max_tool_calls"), Some(48));
    }
}
