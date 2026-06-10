//! Skill install suggestions from starter pack gaps and recent conversation usage.

use crate::db::DashboardDb;
use anyhow::Result;
use serde_json::{json, Value};
use std::collections::HashMap;

pub const STARTER_SKILL_IDS: &[&str] = &[
    "daily-brief",
    "weekly-report",
    "doc-summary",
    "content-repurpose",
    "file-organizer",
    "report-to-csv",
    "md-to-pdf",
    "cn-daily-brief",
    "cn-weekly-report",
    "cn-meeting-minutes",
];

pub async fn build_suggestions(db: &DashboardDb) -> Result<Value> {
    let skills = db.list_skills(200).await?;
    let installed: std::collections::HashSet<String> =
        skills.iter().map(|s| s.id.clone()).collect();
    let missing_starter: Vec<&str> = STARTER_SKILL_IDS
        .iter()
        .copied()
        .filter(|id| !installed.contains(*id))
        .collect();
    let usage = recent_skill_usage(db).await?;
    Ok(json!({
        "missing_starter": missing_starter,
        "usage": usage,
        "installed_count": installed.len(),
    }))
}

#[derive(Debug, serde::Serialize)]
struct SkillUsageRow {
    skill_id: String,
    count: i64,
}

async fn recent_skill_usage(db: &DashboardDb) -> Result<Vec<SkillUsageRow>> {
    let rows = sqlx::query_scalar::<_, String>(
        r#"
        SELECT body FROM project_events
        WHERE event_type = 'user_prompt'
          AND body LIKE '%[Use skills:%'
        ORDER BY occurred_at DESC
        LIMIT 400
        "#,
    )
    .fetch_all(db.pool())
    .await?;

    let mut counts: HashMap<String, i64> = HashMap::new();
    for body in rows {
        for id in parse_skill_hint_ids(&body) {
            *counts.entry(id).or_insert(0) += 1;
        }
    }
    let mut usage: Vec<SkillUsageRow> = counts
        .into_iter()
        .map(|(skill_id, count)| SkillUsageRow { skill_id, count })
        .collect();
    usage.sort_by(|a, b| b.count.cmp(&a.count));
    usage.truncate(12);
    Ok(usage)
}

fn parse_skill_hint_ids(body: &str) -> Vec<String> {
    let Some(rest) = body.split("[Use skills:").nth(1) else {
        return vec![];
    };
    let Some(list) = rest.split(']').next() else {
        return vec![];
    };
    list.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && SkillCatalogish::is_valid(s))
        .collect()
}

mod SkillCatalogish {
    pub fn is_valid(id: &str) -> bool {
        !id.is_empty()
            && id.len() <= 64
            && id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_skill_hint_line() {
        let ids = parse_skill_hint_ids("[Use skills: daily-brief, md-to-pdf]\n\nhello");
        assert_eq!(ids, vec!["daily-brief", "md-to-pdf"]);
    }
}
