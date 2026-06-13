//! Per-agent routing profile editor.

use super::accent_title;
use super::provider_flows::provider_default_model;
use crate::app_config::prompts::{prompt_line, prompt_model_for_google, prompt_model_for_zai};
use crate::app_config::{
    load_anycode_config_resolved, save_anycode_config_resolved, validate_llm_provider, ModelProfile,
};
use crate::i18n::{tr, tr_args};
use anycode_llm::{
    normalize_provider_id, ProviderCatalogEntry, PROVIDER_CATALOG, ROUTING_AGENT_PRESETS,
    ZAI_AUTH_METHODS,
};
use dialoguer::theme::ColorfulTheme;
use dialoguer::{Input, Select};
use fluent_bundle::FluentArgs;
use std::path::PathBuf;

pub(super) async fn run_routing_agents_flow(
    config_file: Option<PathBuf>,
    path: &std::path::Path,
    is_tty: bool,
) -> anyhow::Result<()> {
    let mut existing = load_anycode_config_resolved(config_file.clone())?.ok_or_else(|| {
        let mut a = FluentArgs::new();
        a.set("path", path.display().to_string());
        anyhow::anyhow!("{}", tr_args("wizard-no-config", &a))
    })?;

    accent_title(&tr("model-routing-title"));
    let preset_labels: Vec<String> = ROUTING_AGENT_PRESETS
        .iter()
        .map(|(id, desc)| format!("{} — {}", id, desc))
        .chain(std::iter::once(tr("model-custom-agent")))
        .chain(std::iter::once(tr("model-back-menu")))
        .collect();

    let pi = if is_tty {
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("model-pick-agent-type"))
            .default(0)
            .items(&preset_labels)
            .interact()?
    } else {
        for (i, l) in preset_labels.iter().enumerate() {
            println!("  {}) {}", i + 1, l);
        }
        println!("  0) {}", tr("model-back"));
        loop {
            let v = prompt_line(&tr("model-enter-number"))?;
            if v.trim() == "0" {
                return Ok(());
            }
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= preset_labels.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    if pi >= preset_labels.len() - 1 {
        return Ok(());
    }

    let agent_key = if pi < ROUTING_AGENT_PRESETS.len() {
        ROUTING_AGENT_PRESETS[pi].0.to_string()
    } else if is_tty {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("agent_type")
            .interact_text()?
    } else {
        prompt_line("agent_type: ")?
    };

    let mut ek = FluentArgs::new();
    ek.set("key", agent_key.clone());
    println!("{}", tr_args("model-edit-routing", &ek));

    let cur = existing
        .routing
        .agents
        .get(&agent_key)
        .cloned()
        .unwrap_or_default();

    let def_p = existing.provider.clone();
    let mut kp = FluentArgs::new();
    kp.set("p", def_p.clone());
    println!("{}", tr_args("model-keep-global", &kp));

    let routable: Vec<&ProviderCatalogEntry> = PROVIDER_CATALOG
        .iter()
        .filter(|e| !e.placeholder_only)
        .collect();
    let mut prov_labels: Vec<String> = routable
        .iter()
        .map(|e| format!("{} — {}", e.id, e.label))
        .collect();
    prov_labels.push("(inherit global provider)".to_string());
    prov_labels.push("(type custom provider id)".to_string());

    let p_idx = if is_tty {
        let def_i = cur
            .provider
            .as_deref()
            .map(normalize_provider_id)
            .and_then(|id| routable.iter().position(|e| e.id == id))
            .unwrap_or(prov_labels.len() - 2);
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt("provider for this agent (OpenClaw catalog)")
            .default(def_i.min(prov_labels.len().saturating_sub(1)))
            .items(&prov_labels)
            .interact()?
    } else {
        for (i, l) in prov_labels.iter().enumerate() {
            println!("  {}) {}", i + 1, l);
        }
        loop {
            let v = prompt_line(&tr("model-pick-number"))?;
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= prov_labels.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    let provider = if p_idx < routable.len() {
        Some(routable[p_idx].id.to_string())
    } else if p_idx == routable.len() {
        None
    } else {
        let raw = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("provider id")
                .interact_text()?
        } else {
            prompt_line("provider id: ")?
        };
        let t = raw.trim();
        if t.is_empty() {
            None
        } else {
            let n = normalize_provider_id(t);
            validate_llm_provider(&n)?;
            Some(n)
        }
    };

    let model = if provider.as_deref() == Some("z.ai") || provider.as_deref() == Some("google") {
        let synthetic = Some(existing.clone());
        let m = if provider.as_deref() == Some("z.ai") {
            prompt_model_for_zai(is_tty, &synthetic)?
        } else {
            prompt_model_for_google(is_tty, &synthetic)?
        };
        Some(m)
    } else if provider.is_some() {
        let def_m = cur.model.clone().unwrap_or_else(|| {
            provider_default_model(provider.as_deref().unwrap_or("openai")).to_string()
        });
        let model_in = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("model")
                .default(def_m.clone())
                .allow_empty(true)
                .interact_text()?
        } else {
            prompt_line(&tr("model-prompt-model-skip"))?
        };
        if model_in.trim().is_empty() {
            None
        } else {
            Some(model_in.trim().to_string())
        }
    } else {
        let model_in = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("model")
                .default(cur.model.clone().unwrap_or_default())
                .allow_empty(true)
                .interact_text()?
        } else {
            prompt_line(&tr("model-prompt-model-skip"))?
        };
        if model_in.trim().is_empty() {
            None
        } else {
            Some(model_in.trim().to_string())
        }
    };

    let plan = if provider.as_deref() == Some("z.ai") {
        let plan_labels: Vec<String> = ZAI_AUTH_METHODS
            .iter()
            .map(|z| {
                z.hint
                    .map(|h| format!("{} — {} ({})", z.label, h, z.plan))
                    .unwrap_or_else(|| format!("{} ({})", z.label, z.plan))
            })
            .collect();
        let zi = if is_tty {
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt("z.ai endpoint / plan")
                .default(0)
                .items(&plan_labels)
                .interact()?
        } else {
            for (i, l) in plan_labels.iter().enumerate() {
                println!("  {}) {}", i + 1, l);
            }
            loop {
                let v = prompt_line(&tr("model-pick-number"))?;
                if let Ok(n) = v.trim().parse::<usize>() {
                    if n >= 1 && n <= plan_labels.len() {
                        break n - 1;
                    }
                }
                println!("{}", tr("model-invalid"));
            }
        };
        Some(ZAI_AUTH_METHODS[zi].plan.to_string())
    } else {
        let plan_in = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt("plan（optional; z.ai: coding|general|coding_cn|general_cn）")
                .default(cur.plan.clone().unwrap_or_default())
                .allow_empty(true)
                .interact_text()?
        } else {
            prompt_line(&tr("model-prompt-plan-skip"))?
        };
        if plan_in.trim().is_empty() {
            None
        } else {
            Some(plan_in.trim().to_string())
        }
    };

    let ak_in = if is_tty {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("model-prompt-api-key-profile"))
            .default(cur.api_key.clone().unwrap_or_default())
            .allow_empty(true)
            .interact_text()?
    } else {
        prompt_line(&tr("model-prompt-api-key-skip"))?
    };
    let api_key = if ak_in.trim().is_empty() {
        None
    } else {
        Some(ak_in.trim().to_string())
    };

    let bu_in = if is_tty {
        Input::with_theme(&ColorfulTheme::default())
            .with_prompt("base_url")
            .default(cur.base_url.clone().unwrap_or_default())
            .allow_empty(true)
            .interact_text()?
    } else {
        prompt_line(&tr("model-prompt-base-url-skip"))?
    };
    let base_url = if bu_in.trim().is_empty() {
        None
    } else {
        Some(bu_in.trim().to_string())
    };

    let profile = ModelProfile {
        provider,
        api_key,
        plan,
        model,
        temperature: cur.temperature,
        max_tokens: cur.max_tokens,
        base_url,
    };

    let empty_profile = profile.provider.is_none()
        && profile.api_key.is_none()
        && profile.plan.is_none()
        && profile.model.is_none()
        && profile.base_url.is_none()
        && profile.temperature.is_none()
        && profile.max_tokens.is_none();

    if empty_profile {
        existing.routing.agents.remove(&agent_key);
    } else {
        existing.routing.agents.insert(agent_key, profile);
    }

    save_anycode_config_resolved(config_file, &existing)?;
    let mut ru = FluentArgs::new();
    ru.set("path", path.display().to_string());
    println!("{}", tr_args("model-routing-updated", &ru));
    Ok(())
}
