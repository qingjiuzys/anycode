//! Interactive prompt helpers shared by config/model wizards.

use std::io::{self, Write};

use super::{is_anthropic_family_provider, is_zai_family_provider, AnyCodeConfig};
use crate::i18n::{tr, tr_args};
use anycode_llm::{
    normalize_provider_id, transport_for_provider_id, LlmTransport, GOOGLE_MODEL_CATALOG,
    ZAI_MODEL_CATALOG,
};
use fluent_bundle::FluentArgs;

/// Non-TTY fallback: read one line from stdin after printing `label`.
pub(crate) fn prompt_line(label: &str) -> anyhow::Result<String> {
    print!("{label}");
    io::stdout().flush()?;
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf.trim_end_matches(['\r', '\n']).to_string())
}

pub(crate) fn prompt_model_for_zai(
    is_tty: bool,
    existing: &Option<AnyCodeConfig>,
) -> anyhow::Result<String> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Input, Select};

    let catalog_items: Vec<String> = ZAI_MODEL_CATALOG
        .iter()
        .map(|e| {
            let mut a = FluentArgs::new();
            a.set("api", e.api_name);
            a.set("display", e.display_name);
            tr_args("zai-model-catalog-entry", &a)
        })
        .chain(std::iter::once(tr("zai-model-custom")))
        .collect();

    let default_model = existing
        .as_ref()
        .filter(|c| is_zai_family_provider(&c.provider))
        .map(|c| c.model.clone())
        .unwrap_or_else(|| "glm-5".to_string());

    let idx = if is_tty {
        let default_i = ZAI_MODEL_CATALOG
            .iter()
            .position(|e| e.api_name == default_model.as_str())
            .unwrap_or(0);
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-pick-model-prompt"))
            .default(default_i.min(catalog_items.len().saturating_sub(1)))
            .items(&catalog_items)
            .interact()?
    } else {
        println!("{}", tr("wizard-pick-model-prompt"));
        for (i, label) in catalog_items.iter().enumerate() {
            println!("  {}) {}", i + 1, label);
        }
        loop {
            let v = prompt_line(&tr("model-pick-number"))?;
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= catalog_items.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    if idx < ZAI_MODEL_CATALOG.len() {
        return Ok(ZAI_MODEL_CATALOG[idx].api_name.to_string());
    }

    if is_tty {
        Ok(Input::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-prompt-model-id"))
            .default(default_model)
            .interact_text()?)
    } else {
        let v = prompt_line(&tr("wizard-model-id-non-tty"))?;
        Ok(if v.is_empty() { default_model } else { v })
    }
}

const ANTHROPIC_MODEL_CHOICES: &[(&str, &str)] = &[
    ("claude-sonnet-4-20250514", "Claude Sonnet 4"),
    ("claude-3-5-sonnet-20241022", "Claude 3.5 Sonnet"),
    ("claude-3-opus-20240229", "Claude 3 Opus"),
];

fn is_google_family_provider(p: &str) -> bool {
    matches!(p.trim(), "google" | "gemini")
}

pub(crate) fn prompt_model_for_google(
    is_tty: bool,
    existing: &Option<AnyCodeConfig>,
) -> anyhow::Result<String> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Input, Select};

    let catalog_items: Vec<String> = GOOGLE_MODEL_CATALOG
        .iter()
        .map(|e| format!("{} — {}", e.id, e.label))
        .chain(std::iter::once(tr("zai-model-custom")))
        .collect();

    let default_model = existing
        .as_ref()
        .filter(|c| is_google_family_provider(&c.provider))
        .map(|c| c.model.clone())
        .unwrap_or_else(|| "gemini-2.5-pro".to_string());

    let idx = if is_tty {
        let default_i = GOOGLE_MODEL_CATALOG
            .iter()
            .position(|e| e.id == default_model.as_str())
            .unwrap_or(0);
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-pick-model-prompt"))
            .default(default_i.min(catalog_items.len().saturating_sub(1)))
            .items(&catalog_items)
            .interact()?
    } else {
        println!("{}", tr("wizard-pick-model-prompt"));
        for (i, label) in catalog_items.iter().enumerate() {
            println!("  {}) {}", i + 1, label);
        }
        loop {
            let v = prompt_line(&tr("model-pick-number"))?;
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= catalog_items.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    if idx < GOOGLE_MODEL_CATALOG.len() {
        return Ok(GOOGLE_MODEL_CATALOG[idx].id.to_string());
    }

    if is_tty {
        Ok(Input::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-prompt-model-id"))
            .default(default_model)
            .interact_text()?)
    } else {
        let v = prompt_line(&tr("wizard-model-id-non-tty"))?;
        Ok(if v.is_empty() { default_model } else { v })
    }
}

pub(crate) fn prompt_model_for_anthropic(
    is_tty: bool,
    existing: &Option<AnyCodeConfig>,
) -> anyhow::Result<String> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Input, Select};

    let mut labels: Vec<String> = ANTHROPIC_MODEL_CHOICES
        .iter()
        .map(|(id, title)| {
            let mut a = FluentArgs::new();
            a.set("id", *id);
            a.set("title", *title);
            tr_args("anthropic-model-catalog-entry", &a)
        })
        .collect();
    labels.push(tr("anthropic-model-custom"));

    let default_model = existing
        .as_ref()
        .filter(|c| is_anthropic_family_provider(&c.provider))
        .map(|c| c.model.clone())
        .unwrap_or_else(|| "claude-sonnet-4-20250514".to_string());

    let idx = if is_tty {
        let default_i = ANTHROPIC_MODEL_CHOICES
            .iter()
            .position(|(id, _)| *id == default_model.as_str())
            .unwrap_or(0);
        Select::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-pick-anthropic-prompt"))
            .default(default_i.min(labels.len().saturating_sub(1)))
            .items(&labels)
            .interact()?
    } else {
        println!("{}", tr("wizard-pick-anthropic-prompt"));
        for (i, label) in labels.iter().enumerate() {
            println!("  {}) {}", i + 1, label);
        }
        loop {
            let v = prompt_line(&tr("model-pick-number"))?;
            if let Ok(n) = v.trim().parse::<usize>() {
                if n >= 1 && n <= labels.len() {
                    break n - 1;
                }
            }
            println!("{}", tr("model-invalid"));
        }
    };

    if idx < ANTHROPIC_MODEL_CHOICES.len() {
        return Ok(ANTHROPIC_MODEL_CHOICES[idx].0.to_string());
    }

    if is_tty {
        Ok(Input::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-prompt-model-id"))
            .default(default_model)
            .interact_text()?)
    } else {
        let v = prompt_line(&tr("wizard-model-id-non-tty"))?;
        Ok(if v.is_empty() { default_model } else { v })
    }
}

/// `recommended_url`：用于 Input 默认值；若用户保留该默认，则写入 `None`（由运行时解析官方默认）。
pub(crate) fn prompt_api_key_and_base_url(
    is_tty: bool,
    existing: &Option<AnyCodeConfig>,
    provider_for_merge: &str,
    _plan: &str,
    recommended_url: &str,
    prompt_base_url: bool,
) -> anyhow::Result<(String, Option<String>)> {
    use dialoguer::theme::ColorfulTheme;
    use dialoguer::{Input, Password};

    let default_api_key = existing
        .as_ref()
        .filter(|c| {
            c.provider == provider_for_merge
                || (is_zai_family_provider(provider_for_merge)
                    && is_zai_family_provider(&c.provider))
                || (is_anthropic_family_provider(provider_for_merge)
                    && is_anthropic_family_provider(&c.provider))
        })
        .map(|c| c.api_key.clone())
        .unwrap_or_default();

    let default_base_url = existing
        .as_ref()
        .filter(|c| {
            c.provider == provider_for_merge
                || (is_zai_family_provider(provider_for_merge)
                    && is_zai_family_provider(&c.provider))
                || (is_anthropic_family_provider(provider_for_merge)
                    && is_anthropic_family_provider(&c.provider))
        })
        .and_then(|c| c.base_url.clone())
        .unwrap_or_default();

    accent_line_api_key_prompt();
    let api_key: String = if is_tty {
        // 已有 api_key 时必须允许「空回车」，否则 dialoguer 会反复提示，无法「保留已有」。
        Password::with_theme(&ColorfulTheme::default())
            .with_prompt(tr("wizard-api-key-prompt"))
            .allow_empty_password(!default_api_key.is_empty())
            .interact()?
    } else {
        prompt_line(&tr("wizard-api-key-prompt"))?
    };
    let api_key = if api_key.is_empty() {
        default_api_key
    } else {
        api_key
    }
    .trim()
    .to_string();
    if api_key.is_empty() {
        anyhow::bail!("{}", tr("cfg-api-empty"));
    }

    let base_url = if prompt_base_url {
        accent_line_base_url_prompt();
        let recommended_default = recommended_url.to_string();
        let shown_default = if default_base_url.is_empty() {
            recommended_default.clone()
        } else {
            default_base_url.clone()
        };

        let base_url_in: String = if is_tty {
            Input::with_theme(&ColorfulTheme::default())
                .with_prompt(tr("wizard-base-url-merge-pty"))
                .default(shown_default.clone())
                .interact_text()?
        } else {
            let mut bu = FluentArgs::new();
            bu.set("url", shown_default.clone());
            let v = prompt_line(&tr_args("wizard-base-url-merge-fallback", &bu))?;
            if v.is_empty() {
                shown_default.clone()
            } else {
                v
            }
        };
        normalize_base_url_input(&base_url_in, provider_for_merge, recommended_url)
    } else if !default_base_url.trim().is_empty() {
        normalize_base_url_input(&default_base_url, provider_for_merge, recommended_url)
    } else if !recommended_url.trim().is_empty() {
        normalize_base_url_input(recommended_url, provider_for_merge, recommended_url)
    } else {
        None
    };
    Ok((api_key, base_url))
}

fn accent_line_api_key_prompt() {
    use console::Style;
    println!(
        "{}",
        Style::new()
            .cyan()
            .bold()
            .apply_to(tr("wizard-api-key-prompt"))
    );
}

fn accent_line_base_url_prompt() {
    use console::Style;
    println!(
        "{}",
        Style::new()
            .cyan()
            .bold()
            .apply_to(tr("cfg-accent-base-url"))
    );
}

/// 与推荐默认一致则存 `None`，由 LLM 层使用官方默认。
fn normalize_base_url_input(
    base_url_in: &str,
    provider_for_merge: &str,
    recommended_url: &str,
) -> Option<String> {
    let v = base_url_in.trim();
    if v.is_empty() {
        return None;
    }
    let norm_provider = normalize_provider_id(provider_for_merge);
    let requires_explicit_openai_endpoint = transport_for_provider_id(&norm_provider)
        == LlmTransport::OpenAiChatCompletions
        && norm_provider != "z.ai";
    if v == recommended_url && !requires_explicit_openai_endpoint {
        return None;
    }
    Some(v.to_string())
}
