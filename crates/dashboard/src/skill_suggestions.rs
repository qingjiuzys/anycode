//! Skill install suggestions from starter pack gaps and recorded Skill tool runs.

use crate::db::DashboardDb;
use anyhow::Result;
use serde_json::{json, Value};

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
    "office-pptx",
    "novel-writer",
    "video-script",
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
    let rows = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT skill_id, COUNT(*) AS run_count
        FROM skill_runs
        WHERE status = 'ok'
        GROUP BY skill_id
        ORDER BY run_count DESC
        LIMIT 12
        "#,
    )
    .fetch_all(db.pool())
    .await?;

    let usage: Vec<SkillUsageRow> = rows
        .into_iter()
        .map(|(skill_id, count)| SkillUsageRow { skill_id, count })
        .collect();
    Ok(usage)
}
