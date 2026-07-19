//! Overview work briefing（汇报）— LLM-authored, facts from recent local activity.

use crate::config_patch::read_config_value;
use crate::db::DashboardDb;
use crate::schema::{OverviewBriefing, OverviewStats};
use anycode_core::{CoreError, LLMProvider, Message, MessageContent, MessageRole, ModelConfig};
use anycode_llm::{
    build_llm_client, capability_catalog::ModelCapability, ProviderConfig, ResolvedModelRegistry,
};
use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use serde::Serialize;
use std::collections::BTreeMap;
use std::time::Duration;
use uuid::Uuid;

const MAX_PROJECTS: usize = 6;
const MAX_EVENTS_PER_PROJECT: usize = 8;
const MAX_SESSIONS_PER_PROJECT: usize = 5;
const EVENT_SCAN_LIMIT: i64 = 80;
const SESSION_SCAN_LIMIT: i64 = 40;

#[derive(Debug, Clone, Serialize)]
struct BriefingEventFact {
    title: String,
    event_type: String,
    severity: String,
    occurred_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct BriefingSessionFact {
    title: String,
    status: String,
    trusted_status: String,
    model: String,
    started_at: String,
}

#[derive(Debug, Clone, Serialize)]
struct BriefingProjectFact {
    project_id: String,
    project_name: String,
    sessions: Vec<BriefingSessionFact>,
    events: Vec<BriefingEventFact>,
}

#[derive(Debug, Clone, Serialize)]
struct BriefingFacts {
    window_days: u32,
    lang: String,
    overview: OverviewStats,
    projects: Vec<BriefingProjectFact>,
}

/// Generate a user-facing work briefing (汇报) for the overview panel.
pub async fn generate_overview_briefing(
    db: &DashboardDb,
    days: u32,
    lang: &str,
) -> Result<OverviewBriefing> {
    let days = days.clamp(1, 30);
    let lang = if lang.eq_ignore_ascii_case("en") {
        "en"
    } else {
        "zh"
    };
    let facts = collect_facts(db, days, lang).await?;
    let generated_at = Utc::now().to_rfc3339();

    match try_llm_briefing(&facts).await {
        Ok((markdown, model)) => Ok(OverviewBriefing {
            markdown,
            generation_mode: "llm".into(),
            window_days: days,
            generated_at,
            model: Some(model),
            fallback_reason: None,
        }),
        Err(err) => Ok(OverviewBriefing {
            markdown: template_briefing(&facts),
            generation_mode: "template".into(),
            window_days: days,
            generated_at,
            model: None,
            fallback_reason: Some(err.to_string()),
        }),
    }
}

async fn collect_facts(db: &DashboardDb, days: u32, lang: &str) -> Result<BriefingFacts> {
    let overview = db.overview_stats().await?;
    let events = db.list_recent_events(EVENT_SCAN_LIMIT).await?;
    let sessions = db
        .list_all_sessions(SESSION_SCAN_LIMIT, None, None, None, None, false)
        .await
        .unwrap_or_default();

    let cutoff = Utc::now() - chrono::Duration::days(days as i64);

    let mut by_project: BTreeMap<String, BriefingProjectFact> = BTreeMap::new();
    let mut project_order: Vec<String> = Vec::new();

    for e in events {
        if parse_ts(&e.occurred_at).is_some_and(|t| t < cutoff) {
            continue;
        }
        let entry = by_project.entry(e.project_id.clone()).or_insert_with(|| {
            project_order.push(e.project_id.clone());
            BriefingProjectFact {
                project_id: e.project_id.clone(),
                project_name: e.project_name.clone(),
                sessions: Vec::new(),
                events: Vec::new(),
            }
        });
        if entry.events.len() < MAX_EVENTS_PER_PROJECT {
            entry.events.push(BriefingEventFact {
                title: e.title,
                event_type: e.event_type,
                severity: e.severity,
                occurred_at: e.occurred_at,
            });
        }
    }

    for s in sessions {
        if parse_ts(&s.started_at).is_some_and(|t| t < cutoff) {
            continue;
        }
        let entry = by_project.entry(s.project_id.clone()).or_insert_with(|| {
            project_order.push(s.project_id.clone());
            BriefingProjectFact {
                project_id: s.project_id.clone(),
                project_name: s.project_name.clone(),
                sessions: Vec::new(),
                events: Vec::new(),
            }
        });
        if entry.project_name.is_empty() {
            entry.project_name = s.project_name.clone();
        }
        if entry.sessions.len() < MAX_SESSIONS_PER_PROJECT {
            entry.sessions.push(BriefingSessionFact {
                title: s.title,
                status: s.status,
                trusted_status: s.trusted_status,
                model: s.model,
                started_at: s.started_at,
            });
        }
    }

    let projects: Vec<_> = project_order
        .into_iter()
        .filter_map(|id| by_project.remove(&id))
        .filter(|p| !p.sessions.is_empty() || !p.events.is_empty())
        .take(MAX_PROJECTS)
        .collect();

    Ok(BriefingFacts {
        window_days: days,
        lang: lang.to_string(),
        overview,
        projects,
    })
}

fn parse_ts(raw: &str) -> Option<chrono::DateTime<Utc>> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(trimmed) {
        return Some(dt.with_timezone(&Utc));
    }
    let normalized = if trimmed.contains('T') {
        trimmed.to_string()
    } else {
        trimmed.replace(' ', "T")
    };
    chrono::NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|ndt| ndt.and_utc())
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(&normalized, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|ndt| ndt.and_utc())
        })
}

fn system_prompt(lang: &str) -> String {
    if lang == "en" {
        r#"You write a concise work briefing for an anyCode user about THEIR projects (not product roadmap).
Tone: short managerial work report. No marketing, no emoji, no fluff.
Rules:
- Use ONLY facts from the JSON. Never invent sessions, files, or outcomes.
- Structure:
  1) One-line overall summary
  2) Per-project H2 sections with 2-4 bullet points of what was done / blocked / pending
  3) Optional closing H2 "Next focus" with at most 3 bullets grounded in the facts
- Prefer human wording of titles over raw event_type codes.
- If there is no activity, say so briefly.
Respond with a single JSON object only (no fences):
{"markdown":"..."}"#
            .into()
    } else {
        r#"你是 anyCode 用户的工作汇报助手。读者是用户自己，内容是他们在各项目上最近做了什么——不是 anyCode 产品进展，也不是工程交付验收报告。
语气：简洁的工作汇报。不要营销腔、不要 emoji、不要客套废话。
规则：
- 只能使用用户 JSON 中的事实，禁止编造会话、结果、路径或结论。
- 结构固定：
  1）开头一段总述（1-2 句）
  2）按项目用二级标题分节，每节 2-4 条要点：做成了什么 / 卡在哪里 / 待处理
  3）可选二级标题「下一步关注」最多 3 条，必须能被事实支撑
- 把原始 event_type 翻译成白话；会话标题可保留。
- 若窗口内无活动，一句话说清即可。
只输出一个 JSON 对象（不要代码围栏）：
{"markdown":"..."}"#
            .into()
    }
}

fn user_prompt(facts_json: &str, lang: &str) -> String {
    if lang == "en" {
        format!(
            "Work-activity facts JSON (source of truth):\n{facts_json}\n\nWrite the briefing JSON now."
        )
    } else {
        format!("工作活动事实 JSON（唯一依据）：\n{facts_json}\n\n请现在生成汇报 JSON。")
    }
}

async fn try_llm_briefing(facts: &BriefingFacts) -> Result<(String, String)> {
    let (_, cfg) = read_config_value(None).context("read ~/.anycode/config.json")?;
    let registry = ResolvedModelRegistry::from_config(&cfg);
    let pc = chat_provider_config(&registry)?;
    let client = build_llm_client(&pc)
        .await
        .map_err(|e: CoreError| anyhow!(e.to_string()))?;

    let facts_json = serde_json::to_string(facts).context("serialize briefing facts")?;
    let system = system_prompt(&facts.lang);
    let user = user_prompt(&facts_json, &facts.lang);
    let model_name = pc.model.clone();

    let resp = tokio::time::timeout(
        Duration::from_secs(60),
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
                temperature: Some(0.3),
                max_tokens: Some(2048),
                api_key: Some(pc.api_key.clone()),
                ..Default::default()
            },
        ),
    )
    .await
    .map_err(|_| anyhow!("LLM briefing timed out"))??;

    let text = match resp.message.content {
        MessageContent::Text(t) => t,
        _ => return Err(anyhow!("LLM returned non-text response")),
    };
    let markdown = parse_briefing_markdown(&text)?;
    Ok((markdown, model_name))
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
        temperature: Some(0.3),
        max_tokens: Some(2048),
        zai_tool_choice_first_turn: false,
    })
}

fn parse_briefing_markdown(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    let json_str = extract_json_object(trimmed)?;
    let value: serde_json::Value = serde_json::from_str(json_str).context("parse briefing JSON")?;
    let md = value
        .get("markdown")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if md.is_empty() {
        return Err(anyhow!("LLM returned empty markdown"));
    }
    Ok(md.to_string())
}

fn extract_json_object(raw: &str) -> Result<&str> {
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            if end > start {
                return Ok(&raw[start..=end]);
            }
        }
    }
    Err(anyhow!("no JSON object in LLM response"))
}

fn template_briefing(facts: &BriefingFacts) -> String {
    let zh = facts.lang != "en";
    let mut out = String::new();
    if facts.projects.is_empty() {
        if zh {
            out.push_str(
                "近几日暂无项目活动。\n\n开启会话并完成一些工作后，这里会生成按项目汇总的汇报。\n",
            );
        } else {
            out.push_str("No project activity in this window.\n\nAfter you run sessions, this briefing will summarize progress by project.\n");
        }
        return out;
    }

    if zh {
        out.push_str(&format!(
            "近 {} 天共登记 {} 个项目、{} 次会话（其中运行中 {}、阻断 {}）。\n\n",
            facts.window_days,
            facts.overview.projects_count,
            facts.overview.sessions_total,
            facts.overview.sessions_running,
            facts.overview.sessions_blocked,
        ));
    } else {
        out.push_str(&format!(
            "Last {} day(s): {} projects, {} sessions (running {}, blocked {}).\n\n",
            facts.window_days,
            facts.overview.projects_count,
            facts.overview.sessions_total,
            facts.overview.sessions_running,
            facts.overview.sessions_blocked,
        ));
    }

    for p in &facts.projects {
        out.push_str(&format!("## {}\n\n", p.project_name));
        for s in &p.sessions {
            if zh {
                out.push_str(&format!(
                    "- 会话「{}」：状态 {} / 信任 {}\n",
                    s.title, s.status, s.trusted_status
                ));
            } else {
                out.push_str(&format!(
                    "- Session \"{}\": status {} / trust {}\n",
                    s.title, s.status, s.trusted_status
                ));
            }
        }
        for e in p.events.iter().take(4) {
            out.push_str(&format!("- {}\n", e.title));
        }
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_briefing_json() {
        let md = parse_briefing_markdown("{\"markdown\":\"## A\\n- done\"}").unwrap();
        assert!(md.contains("## A"));
    }

    #[test]
    fn template_empty_projects() {
        let facts = BriefingFacts {
            window_days: 7,
            lang: "zh".into(),
            overview: OverviewStats {
                projects_count: 0,
                sessions_total: 0,
                sessions_running: 0,
                sessions_blocked: 0,
                sessions_budget_exceeded: 0,
                artifacts_count: 0,
                skills_count: 0,
                gates_failed: 0,
                events_last_hour: 0,
            },
            projects: vec![],
        };
        let md = template_briefing(&facts);
        assert!(md.contains("暂无"));
    }
}
