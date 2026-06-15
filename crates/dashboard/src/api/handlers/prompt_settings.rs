use crate::config_patch::{read_config_root, write_config_root};
use anycode_agent::{
    ExploreAgent, GeneralPurposeAgent, PlanAgent, PromptAssembler, RuntimePromptConfig,
};
use anycode_core::{Agent, LLMProvider, ModelConfig};
use anycode_tools::SkillCatalog;
use axum::{extract::Query, http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;

#[derive(Deserialize)]
pub struct PromptPreviewQuery {
    pub agent: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Deserialize)]
pub struct PromptSettingsBody {
    pub system_prompt_append: Option<String>,
    pub system_prompt_override: Option<String>,
}

fn optional_string(v: Option<String>) -> Option<String> {
    v.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub async fn get_prompt_preview(Query(q): Query<PromptPreviewQuery>) -> impl IntoResponse {
    let (_, cfg_json) = match read_config_root() {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let system_prompt_override = cfg_json
        .get("system_prompt_override")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let system_prompt_append = cfg_json
        .get("system_prompt_append")
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let cwd = q.cwd.filter(|s| !s.trim().is_empty()).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".into())
    });

    let mut prompt_config = RuntimePromptConfig::default();
    if let Some(v) = system_prompt_override
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        prompt_config.system_prompt_override = Some(v.to_string());
    }
    if let Some(v) = system_prompt_append
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        prompt_config.system_prompt_append = Some(v.to_string());
    }

    let skills_enabled = cfg_json
        .get("skills")
        .and_then(|s| s.get("enabled"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if skills_enabled {
        let cwd_path = PathBuf::from(&cwd);
        let mut roots = vec![cwd_path.join("skills"), cwd_path.join(".anycode/skills")];
        if let Some(h) = dirs::home_dir() {
            roots.push(h.join(".anycode/skills"));
        }
        let catalog = SkillCatalog::scan(&roots, None, 120_000, false);
        if let Some(section) = catalog.render_prompt_subsection() {
            prompt_config.skills_section = Some(section);
        }
    }

    let agent_id = q
        .agent
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("general-purpose");
    let model_config = ModelConfig {
        provider: LLMProvider::Custom("preview".into()),
        model: "preview".into(),
        base_url: None,
        temperature: None,
        max_tokens: None,
        api_key: None,
        ..Default::default()
    };
    let agent: Box<dyn Agent> = match agent_id {
        "explore" => Box::new(ExploreAgent::new(model_config.clone(), true)),
        "plan" => Box::new(PlanAgent::new(model_config.clone(), true)),
        _ => Box::new(GeneralPurposeAgent::new(model_config)),
    };

    let assembler = PromptAssembler {
        config: &prompt_config,
        agent: agent.as_ref(),
        cwd: &cwd,
        task_append: None,
    };
    let segments: Vec<_> = assembler
        .build_segments()
        .into_iter()
        .map(|s| json!({ "id": s.id, "text": s.text, "chars": s.text.chars().count() }))
        .collect();

    Json(json!({
        "agent": agent_id,
        "cwd": cwd,
        "system_prompt_override": system_prompt_override,
        "system_prompt_append": system_prompt_append,
        "segments": segments,
        "composed": assembler.compose(),
    }))
    .into_response()
}

pub async fn put_prompt_settings(Json(body): Json<PromptSettingsBody>) -> impl IntoResponse {
    let (_, mut cfg_json) = match read_config_root() {
        Ok(v) => v,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };
    let obj = match cfg_json.as_object_mut() {
        Some(o) => o,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "config root is not an object" })),
            )
                .into_response();
        }
    };
    set_or_remove_string_field(obj, "system_prompt_append", body.system_prompt_append);
    set_or_remove_string_field(obj, "system_prompt_override", body.system_prompt_override);
    let saved_append = obj.get("system_prompt_append").cloned();
    let saved_override = obj.get("system_prompt_override").cloned();
    match write_config_root(&cfg_json) {
        Ok(path) => Json(json!({
            "ok": true,
            "config_path": path.display().to_string(),
            "system_prompt_append": saved_append,
            "system_prompt_override": saved_override,
            "restart_hint": "Start a new conversation for prompt changes to apply."
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

fn set_or_remove_string_field(
    obj: &mut serde_json::Map<String, Value>,
    key: &str,
    value: Option<String>,
) {
    match optional_string(value) {
        Some(v) => {
            obj.insert(key.to_string(), Value::String(v));
        }
        None => {
            obj.remove(key);
        }
    }
}
