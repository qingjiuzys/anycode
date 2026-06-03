//! Agent profile CRUD API.

use super::*;
use crate::db::UpsertAgentProfileRequest;
use anycode_agent::{apply_tool_filters, base_tools_for_extends, is_builtin_extends};
use serde_json::{json, Value};

pub async fn list_agent_profiles(State(state): State<AppState>) -> impl IntoResponse {
    let _ = state.db.seed_builtin_agent_profiles().await;
    match state.db.list_agent_profiles().await {
        Ok(rows) => Json(json!({ "profiles": rows })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_agent_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_agent_profile(&id).await {
        Ok(Some(row)) => Json(json!({ "profile": row })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "profile not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn put_agent_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpsertAgentProfileRequest>,
) -> impl IntoResponse {
    if is_builtin_extends(&id) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "id conflicts with builtin agent" })),
        )
            .into_response();
    }
    if let Err(e) = patch_config_agent_profile(&id, &body) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("config write failed: {e}") })),
        )
            .into_response();
    }
    match state.db.upsert_agent_profile(&id, &body, false).await {
        Ok(row) => Json(json!({ "profile": row })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_agent_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = delete_config_agent_profile(&id) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("config delete failed: {e}") })),
        )
            .into_response();
    }
    match state.db.delete_agent_profile(&id).await {
        Ok(true) => Json(json!({ "ok": true })).into_response(),
        Ok(false) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "profile not found or builtin" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_agent_profile_effective(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let profile = match state.db.get_agent_profile(&id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "profile not found" })),
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
    let extends = if profile.extends.trim().is_empty() {
        "general-purpose".to_string()
    } else {
        profile.extends.clone()
    };
    let tools_json: Value = serde_json::from_str(&profile.tools_json).unwrap_or(json!({}));
    let allow = tools_json.get("allow").and_then(|v| v.as_array()).map(|a| {
        a.iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect::<Vec<_>>()
    });
    let deny = tools_json.get("deny").and_then(|v| v.as_array()).map(|a| {
        a.iter()
            .filter_map(|x| x.as_str().map(String::from))
            .collect::<Vec<_>>()
    });
    let base = base_tools_for_extends(&extends, false);
    let tools = apply_tool_filters(base, allow.as_deref(), deny.as_deref());
    Json(json!({
        "id": profile.id,
        "extends": extends,
        "tools": tools,
        "skills_json": serde_json::from_str::<Value>(&profile.skills_json).unwrap_or(json!({})),
        "routing_json": serde_json::from_str::<Value>(&profile.routing_json).unwrap_or(json!({})),
    }))
    .into_response()
}

fn patch_config_agent_profile(id: &str, req: &UpsertAgentProfileRequest) -> anyhow::Result<()> {
    let (_, mut root) = crate::config_patch::read_config_root()?;
    let agents = root
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("config root not object"))?;
    let entry = agents
        .entry("agents")
        .or_insert(json!({ "profiles": {}, "defaults": {} }));
    let profiles = entry
        .get_mut("profiles")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| anyhow::anyhow!("agents.profiles missing"))?;
    profiles.insert(
        id.to_string(),
        json!({
            "extends": req.extends,
            "description": req.description,
            "tools": req.tools_json,
            "skills": req.skills_json,
            "routing": req.routing_json,
            "prompt_overlay": req.prompt_overlay,
        }),
    );
    crate::config_patch::write_config_root(&root)?;
    Ok(())
}

fn delete_config_agent_profile(id: &str) -> anyhow::Result<()> {
    let (_, mut root) = crate::config_patch::read_config_root()?;
    if let Some(profiles) = root
        .get_mut("agents")
        .and_then(|a| a.get_mut("profiles"))
        .and_then(|p| p.as_object_mut())
    {
        profiles.remove(id);
    }
    crate::config_patch::write_config_root(&root)?;
    Ok(())
}
