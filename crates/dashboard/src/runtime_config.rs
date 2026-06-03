//! Read-only runtime settings surfaced to the dashboard Settings UI.

use crate::schema::{AssetReadPolicySummary, RoutingAgentEntry, RuntimeSettings};
use crate::service_governance::is_loopback_host;
use anycode_llm::{read_config_value, read_model_fallback, string_field};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn load_config() -> (PathBuf, Option<Value>) {
    match read_config_value(None) {
        Ok((path, cfg)) => {
            let present = cfg.is_object() && !cfg.as_object().is_some_and(|o| o.is_empty());
            (path, if present { Some(cfg) } else { None })
        }
        Err(_) => (anycode_llm::default_config_path(), None),
    }
}

fn parse_llm_config(
    cfg: Option<&Value>,
) -> (
    Option<String>,
    Option<String>,
    Vec<RoutingAgentEntry>,
    Option<Value>,
) {
    let Some(cfg) = cfg else {
        return (None, None, Vec::new(), None);
    };
    let global_provider = string_field(cfg, "provider", "provider");
    let global_model = string_field(cfg, "model", "model");

    let mut routing_agents = Vec::new();
    if let Some(agents) = cfg
        .get("routing")
        .and_then(|r| r.get("agents"))
        .and_then(|a| a.as_object())
    {
        for (agent, profile) in agents {
            routing_agents.push(RoutingAgentEntry {
                agent: agent.clone(),
                provider: profile
                    .get("provider")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                model: profile
                    .get("model")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            });
        }
        routing_agents.sort_by(|a, b| a.agent.cmp(&b.agent));
    }

    let model_routes = cfg
        .get("runtime")
        .and_then(|r| r.get("model_routes"))
        .cloned();

    (global_provider, global_model, routing_agents, model_routes)
}

fn asset_read_policy_summary(strict: bool) -> AssetReadPolicySummary {
    let mut rules = vec![
        "Dashboard is read-only — no shell execution or file writes from WebUI.".into(),
        "Artifact index tracks FileWrite/Edit/NotebookEdit outputs with session provenance.".into(),
        "Unverified artifacts and blocked sessions cannot satisfy delivery readiness.".into(),
        "External connector configs store summaries only — no secrets in SQLite.".into(),
    ];
    if strict {
        rules.insert(
            0,
            "Strict mode: paths outside the project workspace require explicit approval before index display.".into(),
        );
    }
    AssetReadPolicySummary {
        summary: if strict {
            "Strict asset read policy is enabled in dashboard preferences — unverified external paths are flagged aggressively.".into()
        } else {
            "Project workspace files are readable by default; paths outside the project root, external connectors, and knowledge bases require explicit policy and are audited.".into()
        },
        rules,
    }
}

pub fn build_runtime_settings(
    host: &str,
    port: u16,
    db_path: &Path,
    skills_total: i64,
    skills_enabled_links: i64,
    prefs: Option<&crate::schema::DashboardPreferences>,
) -> RuntimeSettings {
    let (config_path, cfg) = load_config();
    let (global_provider, global_model, routing_agents, model_routes) =
        parse_llm_config(cfg.as_ref());
    let (fb_provider, fb_model) = cfg
        .as_ref()
        .map(read_model_fallback)
        .map(|fb| (fb.provider.clone(), fb.model.clone()))
        .unwrap_or((None, None));
    let asset_strict = prefs.map(|p| p.asset_read_strict).unwrap_or(false);
    let auth_mode = if is_loopback_host(host) {
        "local_trusted"
    } else {
        "token_required"
    };

    RuntimeSettings {
        config_path: config_path.display().to_string(),
        config_present: cfg.is_some(),
        global_provider,
        global_model,
        routing_agents,
        model_routes,
        auth_mode: auth_mode.into(),
        host: host.into(),
        port,
        db_path: db_path.display().to_string(),
        sse_events_path: "/api/events/stream".into(),
        sse_project_events_path: "/api/projects/{id}/events/stream".into(),
        asset_read_policy: asset_read_policy_summary(asset_strict),
        skills_total,
        skills_enabled_links,
        asset_read_strict: asset_strict,
        fallback_provider: fb_provider,
        fallback_model: fb_model,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_without_config_is_ok() {
        let s = build_runtime_settings("127.0.0.1", 43180, Path::new("/tmp/x.db"), 0, 0, None);
        assert_eq!(s.auth_mode, "local_trusted");
        assert!(!s.config_path.is_empty());
    }
}
