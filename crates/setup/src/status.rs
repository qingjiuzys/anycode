use crate::config::has_usable_model_config;
use crate::workspace::workspace_root;
use anycode_llm::read_config_value;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetupStepId {
    Workspace,
    Llm,
    LlmTest,
    Memory,
    Skills,
    Channels,
    Projects,
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStepStatus {
    pub id: SetupStepId,
    pub complete: bool,
    pub optional: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupStatus {
    pub ready: bool,
    pub config_path: String,
    pub platform: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub setup_completed_at: Option<String>,
    pub steps: Vec<SetupStepStatus>,
}

fn platform_label() -> String {
    if cfg!(target_os = "macos") {
        "macos".into()
    } else if cfg!(target_os = "windows") {
        "windows".into()
    } else if cfg!(target_os = "linux") {
        "linux".into()
    } else {
        "unknown".into()
    }
}

fn workspace_ready() -> bool {
    workspace_root().join("projects").is_dir()
}

fn memory_configured(cfg: &Value) -> bool {
    cfg.get("memory")
        .and_then(|m| m.get("backend"))
        .and_then(|b| b.as_str())
        .is_some_and(|s| !s.trim().is_empty())
}

fn starter_skills_installed() -> bool {
    dirs::home_dir()
        .map(|h| h.join(".anycode/skills"))
        .is_some_and(|p| {
            p.is_dir()
                && std::fs::read_dir(p)
                    .map(|mut d| d.next().is_some())
                    .unwrap_or(false)
        })
}

pub fn build_setup_status(
    config: Option<&Value>,
    config_path: &Path,
    setup_completed_at: Option<&str>,
    projects_count: i64,
) -> SetupStatus {
    let cfg = config.cloned().unwrap_or(Value::Object(Default::default()));
    let llm_ok = has_usable_model_config(&cfg);
    let ws_ok = workspace_ready();
    let mem_ok = memory_configured(&cfg);
    let skills_ok = starter_skills_installed();
    let channels_ok = crate::channels::channel_credentials_status().any_configured;
    let projects_ok = projects_count > 0;

    let steps = vec![
        SetupStepStatus {
            id: SetupStepId::Workspace,
            complete: ws_ok,
            optional: false,
        },
        SetupStepStatus {
            id: SetupStepId::Llm,
            complete: llm_ok,
            optional: false,
        },
        SetupStepStatus {
            id: SetupStepId::LlmTest,
            complete: llm_ok,
            optional: false,
        },
        SetupStepStatus {
            id: SetupStepId::Memory,
            complete: mem_ok,
            optional: false,
        },
        SetupStepStatus {
            id: SetupStepId::Skills,
            complete: skills_ok,
            optional: true,
        },
        SetupStepStatus {
            id: SetupStepId::Channels,
            complete: channels_ok,
            optional: true,
        },
        SetupStepStatus {
            id: SetupStepId::Projects,
            complete: projects_ok,
            optional: false,
        },
        SetupStepStatus {
            id: SetupStepId::Done,
            complete: setup_completed_at.is_some() || llm_ok,
            optional: false,
        },
    ];

    SetupStatus {
        ready: llm_ok,
        config_path: config_path.display().to_string(),
        platform: platform_label(),
        setup_completed_at: setup_completed_at.map(str::to_string),
        steps,
    }
}

/// Load config from disk and build setup status.
pub fn load_setup_status(setup_completed_at: Option<&str>, projects_count: i64) -> SetupStatus {
    let (path, cfg) = read_config_value(None).unwrap_or_else(|_| {
        (
            anycode_llm::default_config_path(),
            Value::Object(Default::default()),
        )
    });
    let present = cfg.is_object() && !cfg.as_object().is_some_and(|o| o.is_empty());
    build_setup_status(
        if present { Some(&cfg) } else { None },
        &path,
        setup_completed_at,
        projects_count,
    )
}
