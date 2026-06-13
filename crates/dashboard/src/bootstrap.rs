//! Home/bootstrap summary and onboarding hints.

use crate::db::DashboardDb;
use crate::schema::BootstrapSummary;
use anyhow::Result;

pub async fn bootstrap_summary(
    db: &DashboardDb,
    workspace_paths: &[String],
) -> Result<BootstrapSummary> {
    let stats = db.overview_stats().await?;
    let has_data = stats.projects_count > 0 || stats.sessions_total > 0;

    let mut next_steps = Vec::new();
    if !has_data {
        next_steps
            .push("Run `anycode run` or `anycode goal` in a project to register sessions".into());
        next_steps.push("Or click **Scan projects** on Projects to import recent task logs".into());
        next_steps.push("Start the dashboard: `anycode dashboard --open`".into());
    }
    if stats.gates_failed > 0 {
        next_steps.push(format!(
            "{} required gate(s) failed — open Projects to review blocked sessions",
            stats.gates_failed
        ));
    }
    if stats.sessions_running > 0 {
        next_steps.push(format!(
            "{} session(s) running — check Conversations for live updates",
            stats.sessions_running
        ));
    }

    let workspace_registered = if workspace_paths.is_empty() {
        None
    } else {
        let mut registered = Vec::new();
        for path in workspace_paths {
            let norm = path.trim();
            if norm.is_empty() {
                continue;
            }
            let exists: bool =
                sqlx::query_scalar("SELECT COUNT(*) > 0 FROM projects WHERE root_path = ?")
                    .bind(norm)
                    .fetch_one(db.pool())
                    .await
                    .unwrap_or(false);
            registered.push((norm.to_string(), exists));
        }
        Some(registered)
    };

    Ok(BootstrapSummary {
        has_data,
        projects_count: stats.projects_count,
        sessions_total: stats.sessions_total,
        next_steps,
        workbench_phase: "v3_week10".into(),
        planning_doc: "docs/workbench/digital-workbench-next-steps-zh.md".into(),
        workspace_registered,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}
