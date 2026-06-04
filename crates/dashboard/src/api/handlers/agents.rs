//! Agent profile CRUD API.

use super::*;
use crate::db::{AgentProfileRecord, UpsertAgentProfileRequest};
use anycode_agent::{is_builtin_extends, resolve_profile, AgentProfileSpec};
use serde_json::{json, Value};

pub async fn list_agent_profiles(State(state): State<AppState>) -> impl IntoResponse {
    let _ = state.db.seed_builtin_agent_profiles().await;
    if let Err(e) = sync_config_profiles_to_db(&state.db).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
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
    let _ = state.db.seed_builtin_agent_profiles().await;
    if let Err(e) = sync_config_profiles_to_db(&state.db).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
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
    let row = match state.db.upsert_agent_profile(&id, &body, false).await {
        Ok(row) => row,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    if let Err(e) = patch_config_agent_profile(&id, &body) {
        let _ = state.db.delete_agent_profile(&id).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("config write failed: {e}") })),
        )
            .into_response();
    }
    Json(json!({ "profile": row })).into_response()
}

pub async fn delete_agent_profile(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.db.delete_agent_profile(&id).await {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "profile not found or builtin" })),
            )
                .into_response()
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    }
    if let Err(e) = delete_config_agent_profile(&id) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": format!("config delete failed: {e}") })),
        )
            .into_response();
    }
    Json(json!({ "ok": true })).into_response()
}

pub async fn get_agent_profile_effective(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let _ = state.db.seed_builtin_agent_profiles().await;
    if let Err(e) = sync_config_profiles_to_db(&state.db).await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response();
    }
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
    let spec = profile_spec_from_record(&profile);
    let resolved = resolve_profile(&id, &spec, false);
    Json(json!({
        "id": profile.id,
        "extends": resolved.extends,
        "tools": resolved.tools,
        "skills_json": serde_json::from_str::<Value>(&profile.skills_json).unwrap_or(json!({})),
        "routing_json": serde_json::from_str::<Value>(&profile.routing_json).unwrap_or(json!({})),
        "prompt_overlay": resolved.prompt_overlay,
        "runtime_mode": format!("{:?}", resolved.runtime_mode),
    }))
    .into_response()
}

fn profile_spec_from_record(profile: &AgentProfileRecord) -> AgentProfileSpec {
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
    let skills_json: Value = serde_json::from_str(&profile.skills_json).unwrap_or(json!({}));
    let skills_allowlist = skills_json
        .get("allowlist")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect::<Vec<_>>()
        });
    AgentProfileSpec {
        extends: profile.extends.clone(),
        description: Some(profile.description.clone()),
        tools_allow: allow,
        tools_deny: deny,
        skills_allowlist,
        prompt_overlay: if profile.prompt_overlay.trim().is_empty() {
            None
        } else {
            Some(profile.prompt_overlay.clone())
        },
    }
}

async fn sync_config_profiles_to_db(db: &crate::db::DashboardDb) -> anyhow::Result<()> {
    let (_, root) = crate::config_patch::read_config_root()?;
    let Some(profiles) = root
        .get("agents")
        .and_then(|a| a.get("profiles"))
        .and_then(|p| p.as_object())
    else {
        return Ok(());
    };
    for (id, value) in profiles {
        if is_builtin_extends(id) {
            continue;
        }
        let extends = value
            .get("extends")
            .and_then(|v| v.as_str())
            .unwrap_or("general-purpose")
            .to_string();
        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let tools_json = value.get("tools").cloned();
        let skills_json = value.get("skills").cloned();
        let routing_json = value.get("routing").cloned();
        let prompt_overlay = value
            .get("prompt_overlay")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        db.upsert_agent_profile(
            id,
            &UpsertAgentProfileRequest {
                extends,
                description,
                tools_json,
                skills_json,
                routing_json,
                prompt_overlay,
                scope: Some("global".into()),
                project_id: None,
            },
            false,
        )
        .await?;
    }
    Ok(())
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
