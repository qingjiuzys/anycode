//! LLM-authored report bodies (facts constrained by snapshot JSON).

use crate::config_patch::read_config_value;
use crate::report::html::{escape_html, wrap_fragment};
use crate::report::locale::Lang;
use crate::report::snapshot::ReportSnapshot;
use anycode_core::{CoreError, LLMProvider, Message, MessageContent, MessageRole, ModelConfig};
use anycode_llm::{
    build_llm_client, capability_catalog::ModelCapability, ProviderConfig, ResolvedModelRegistry,
};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::Deserialize;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct LlmReportOutput {
    markdown_body: String,
    #[serde(default)]
    html_body: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LlmReportBodies {
    pub markdown: String,
    pub html: Option<String>,
}

pub async fn try_llm_bodies(snap: &ReportSnapshot, output_format: &str) -> Result<LlmReportBodies> {
    let (_, cfg) = read_config_value(None).context("read ~/.anycode/config.json")?;
    let registry = ResolvedModelRegistry::from_config(&cfg);
    let pc = chat_provider_config(&registry)?;
    let client = build_llm_client(&pc)
        .await
        .map_err(|e: CoreError| anyhow!(e.to_string()))?;

    let snapshot_json = serde_json::to_string(snap).context("serialize report snapshot for LLM")?;
    let want_html = output_format == "html" || output_format == "both";
    let system = system_prompt(snap.lang, want_html);
    let user = user_prompt(&snapshot_json, want_html);

    let resp = tokio::time::timeout(
        Duration::from_secs(90),
        client.chat(
            vec![
                Message {
                    id: Uuid::new_v4(),
                    role: MessageRole::System,
                    content: MessageContent::Text(system),
                    timestamp: Utc::now(),
                    metadata: Default::default(),
                },
                Message {
                    id: Uuid::new_v4(),
                    role: MessageRole::User,
                    content: MessageContent::Text(user),
                    timestamp: Utc::now(),
                    metadata: Default::default(),
                },
            ],
            vec![],
            &ModelConfig {
                provider: LLMProvider::Custom(pc.provider.clone()),
                model: pc.model.clone(),
                base_url: pc.base_url.clone(),
                temperature: Some(0.2),
                max_tokens: Some(8192),
                api_key: Some(pc.api_key.clone()),
            },
        ),
    )
    .await
    .map_err(|_| anyhow!("LLM report generation timed out"))??;

    let text = match resp.message.content {
        MessageContent::Text(t) => t,
        _ => return Err(anyhow!("LLM returned non-text response")),
    };
    parse_llm_output(&text, &snap.title, want_html)
}

fn chat_provider_config(registry: &ResolvedModelRegistry) -> Result<ProviderConfig> {
    let item = registry
        .active_item(ModelCapability::Chat)
        .ok_or_else(|| anyhow!("chat model not configured"))?;
    let api_key = registry
        .resolve_api_key(item)
        .ok_or_else(|| anyhow!("api_key not configured"))?;
    Ok(ProviderConfig {
        provider: registry.resolve_provider(item),
        api_key,
        base_url: registry.resolve_base_url(item),
        model: registry.resolve_model(item),
        temperature: Some(0.2),
        max_tokens: Some(8192),
        zai_tool_choice_first_turn: false,
    })
}

fn system_prompt(lang: Lang, want_html: bool) -> String {
    let lang_note = match lang {
        Lang::Zh => "Write in Chinese (简体).",
        Lang::En => "Write in English.",
    };
    let html_note = if want_html {
        "Include html_body with safe HTML fragments (section, p, table, thead, tbody, tr, th, td, ul, li, code, pre only — no script, style tags, or inline handlers)."
    } else {
        "Set html_body to null or omit it."
    };
    format!(
        r#"You produce internal engineering delivery reports for anycode Digital Workbench.
{lang_note}

Rules:
- Use ONLY facts from the user JSON snapshot. Never invent counts, paths, gate names, or session IDs.
- Tone: neutral internal record. No marketing, no emoji, no filler ("I hope this helps", "great news", etc.).
- Markdown: minimal emphasis; avoid **bold** labels; use ## headings and markdown tables for lists.
- Do not use blockquotes for the main conclusion.
- {html_note}

Respond with a single JSON object only (no markdown fences around the JSON):
{{"markdown_body":"...","html_body":"..." or null}}"#
    )
}

fn user_prompt(snapshot_json: &str, want_html: bool) -> String {
    format!(
        "Snapshot JSON (source of truth):\n{snapshot_json}\n\nOutput format preference: {}\nGenerate the report JSON now.",
        if want_html { "markdown and html" } else { "markdown only" }
    )
}

fn parse_llm_output(raw: &str, title: &str, want_html: bool) -> Result<LlmReportBodies> {
    let trimmed = raw.trim();
    let json_str = extract_json_object(trimmed)?;
    let parsed: LlmReportOutput =
        serde_json::from_str(json_str).context("parse LLM report JSON")?;
    if parsed.markdown_body.trim().is_empty() {
        return Err(anyhow!("LLM returned empty markdown_body"));
    }
    let html = parsed
        .html_body
        .filter(|h| !h.trim().is_empty())
        .map(|body| {
            if body.contains("<!DOCTYPE") || body.contains("<html") {
                body
            } else {
                wrap_fragment(&sanitize_html_fragment(&body), title)
            }
        });
    if want_html && html.is_none() {
        return Err(anyhow!("LLM did not return html_body"));
    }
    Ok(LlmReportBodies {
        markdown: parsed.markdown_body,
        html,
    })
}

fn extract_json_object(s: &str) -> Result<&str> {
    if let Some(start) = s.find('{') {
        if let Some(end) = s.rfind('}') {
            if end > start {
                return Ok(&s[start..=end]);
            }
        }
    }
    Err(anyhow!("no JSON object in LLM response"))
}

fn sanitize_html_fragment(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if lower.contains("<script") || lower.contains("javascript:") || lower.contains("onerror=") {
        return format!("<p>{}</p>", escape_html(raw));
    }
    raw.to_string()
}
