//! Project metrics and delivery readiness aggregates.

use crate::db::DashboardDb;
use crate::observability::project_trust::{readiness_from_inputs, readiness_score};
use crate::schema::{DeliveryReadiness, ProjectMetrics, ProjectReadinessItem};
use anyhow::Result;

pub async fn global_readiness(db: &DashboardDb) -> Result<DeliveryReadiness> {
    let overview = db.overview_stats().await?;
    let blocked = overview.sessions_blocked;
    let failed_gates = overview.gates_failed;
    let running = overview.sessions_running;

    let unverified: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM artifacts WHERE trust_level IN ('unknown', 'needs_verify', 'unverified')"#,
    )
    .fetch_one(db.pool())
    .await?;

    let stale_running: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM sessions
        WHERE status = 'running'
          AND datetime(started_at) < datetime('now', '-24 hours')
        "#,
    )
    .fetch_one(db.pool())
    .await?;

    let mut project_items = Vec::new();
    let projects = db.list_projects().await?;
    for p in projects.iter().take(20) {
        if let Ok(m) = project_metrics(db, &p.id).await {
            let score = readiness_score(
                m.blocked_sessions,
                m.failed_required_gates,
                m.unverified_artifacts,
                m.stale_running_sessions,
            );
            if score < 100 {
                project_items.push(ProjectReadinessItem {
                    project_id: p.id.clone(),
                    project_name: p.name.clone(),
                    readiness_score: score,
                    blocked_sessions: m.blocked_sessions,
                    failed_gates: m.failed_required_gates,
                    unverified_artifacts: m.unverified_artifacts,
                });
            }
        }
    }
    project_items.sort_by_key(|i| i.readiness_score);

    let status = if blocked > 0 || failed_gates > 0 {
        "warn"
    } else if stale_running > 0 || unverified > 0 {
        "warn"
    } else {
        "ok"
    };

    Ok(DeliveryReadiness {
        status: status.into(),
        blocked_sessions: blocked,
        failed_required_gates: failed_gates,
        unverified_artifacts: unverified,
        stale_running_sessions: stale_running,
        running_sessions: running,
        projects: project_items,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub async fn project_metrics(db: &DashboardDb, project_id: &str) -> Result<ProjectMetrics> {
    let trust_inputs = db.fetch_project_trust_inputs(project_id).await?;
    let sessions_total = trust_inputs.sessions_total;
    let blocked_sessions = trust_inputs.blocked_sessions;
    let failed_required_gates = trust_inputs.failed_required_gates;
    let unverified_artifacts = trust_inputs.unverified_artifacts;
    let stale_running_sessions = trust_inputs.stale_running_sessions;

    let sessions_completed: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sessions WHERE project_id = ? AND status IN ('completed', 'done')",
    )
    .bind(project_id)
    .fetch_one(db.pool())
    .await?;

    let events_7d: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM project_events WHERE project_id = ? AND datetime(occurred_at) >= datetime('now', '-7 days')"#,
    )
    .bind(project_id)
    .fetch_one(db.pool())
    .await?;

    let gates_passed: i64 = sqlx::query_scalar(
        r#"SELECT COUNT(*) FROM gates WHERE project_id = ? AND status = 'passed'"#,
    )
    .bind(project_id)
    .fetch_one(db.pool())
    .await?;

    let gates_total = trust_inputs.gates_total;

    let gate_pass_rate = if gates_total > 0 {
        (gates_passed as f64) / (gates_total as f64)
    } else {
        1.0
    };

    let success_rate = if sessions_total > 0 {
        (sessions_completed as f64) / (sessions_total as f64)
    } else {
        0.0
    };

    let readiness_score = readiness_from_inputs(&trust_inputs);

    Ok(ProjectMetrics {
        project_id: project_id.into(),
        sessions_total,
        sessions_completed,
        blocked_sessions,
        failed_required_gates,
        unverified_artifacts,
        stale_running_sessions,
        events_7d,
        gate_pass_rate,
        session_success_rate: success_rate,
        readiness_score,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Rolling daily aggregates for the home timeline chart (computed live from SQLite).
pub async fn global_timeline(
    db: &DashboardDb,
    days: u32,
) -> Result<crate::schema::GlobalTimelineMetrics> {
    use crate::schema::TimelineMetricPoint;
    let days = days.clamp(1, 30);
    let span = format!("-{days} days");

    let session_rows = sqlx::query(
        r#"
        SELECT date(started_at) AS d, COUNT(*) AS c
        FROM sessions
        WHERE datetime(started_at) >= datetime('now', ?)
        GROUP BY date(started_at)
        "#,
    )
    .bind(span.clone())
    .fetch_all(db.pool())
    .await?;

    let event_rows = sqlx::query(
        r#"
        SELECT date(occurred_at) AS d, COUNT(*) AS c
        FROM project_events
        WHERE datetime(occurred_at) >= datetime('now', ?)
        GROUP BY date(occurred_at)
        "#,
    )
    .bind(span.clone())
    .fetch_all(db.pool())
    .await?;

    let gate_rows = sqlx::query(
        r#"
        SELECT date(COALESCE(ended_at, started_at)) AS d, COUNT(*) AS c
        FROM gates
        WHERE status = 'failed'
          AND datetime(COALESCE(ended_at, started_at)) >= datetime('now', ?)
        GROUP BY date(COALESCE(ended_at, started_at))
        "#,
    )
    .bind(span)
    .fetch_all(db.pool())
    .await?;

    use sqlx::Row;
    use std::collections::BTreeMap;

    let mut map: BTreeMap<String, TimelineMetricPoint> = BTreeMap::new();

    for r in session_rows {
        let d: String = r.get("d");
        map.entry(d.clone())
            .or_insert(TimelineMetricPoint {
                date: d,
                sessions_count: 0,
                events_count: 0,
                gates_failed: 0,
            })
            .sessions_count = r.get::<i64, _>("c");
    }
    for r in event_rows {
        let d: String = r.get("d");
        map.entry(d.clone())
            .or_insert(TimelineMetricPoint {
                date: d,
                sessions_count: 0,
                events_count: 0,
                gates_failed: 0,
            })
            .events_count = r.get::<i64, _>("c");
    }
    for r in gate_rows {
        let d: String = r.get("d");
        map.entry(d.clone())
            .or_insert(TimelineMetricPoint {
                date: d,
                sessions_count: 0,
                events_count: 0,
                gates_failed: 0,
            })
            .gates_failed = r.get::<i64, _>("c");
    }

    let points: Vec<TimelineMetricPoint> = map.into_values().collect();
    let trust_trend_pct = if points.len() >= 2 {
        let first = points.first().map(|p| p.sessions_count).unwrap_or(0).max(1) as f64;
        let last = points.last().map(|p| p.sessions_count).unwrap_or(0) as f64;
        ((last - first) / first) * 100.0
    } else {
        0.0
    };

    Ok(crate::schema::GlobalTimelineMetrics {
        days,
        points,
        trust_trend_pct,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

/// Aggregate LLM token usage from `llm_response_end` event payloads.
pub async fn global_token_usage(
    db: &DashboardDb,
    days: u32,
) -> Result<crate::schema::TokenUsageStats> {
    Ok(global_token_usage_detail(db, days).await?.usage)
}

/// Global usage with per-model breakdown (V3).
pub async fn global_token_usage_detail(
    db: &DashboardDb,
    days: u32,
) -> Result<crate::schema::TokenUsageDetail> {
    usage_detail(db, days, None).await
}

/// Per-project usage with model breakdown (V3).
pub async fn project_token_usage_detail(
    db: &DashboardDb,
    project_id: &str,
    days: u32,
) -> Result<crate::schema::TokenUsageDetail> {
    usage_detail(db, days, Some(project_id)).await
}

async fn usage_detail(
    db: &DashboardDb,
    days: u32,
    project_id: Option<&str>,
) -> Result<crate::schema::TokenUsageDetail> {
    use crate::schema::{TokenUsageDetail, TokenUsageStats};
    let days = days.clamp(1, 90);
    let by_model = usage_by_model(db, days, project_id).await?;
    let llm_calls: i64 = by_model.iter().map(|r| r.llm_calls).sum();
    let input_tokens: i64 = by_model.iter().map(|r| r.input_tokens).sum();
    let output_tokens: i64 = by_model.iter().map(|r| r.output_tokens).sum();
    let estimated_cost_usd: f64 = by_model.iter().map(|r| r.estimated_cost_usd).sum();
    Ok(TokenUsageDetail {
        usage: TokenUsageStats {
            days,
            llm_calls,
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost_usd,
            generated_at: chrono::Utc::now().to_rfc3339(),
        },
        by_model,
    })
}

async fn usage_by_model(
    db: &DashboardDb,
    days: u32,
    project_id: Option<&str>,
) -> Result<Vec<crate::schema::ModelUsageRow>> {
    use crate::schema::ModelUsageRow;
    use sqlx::Row;
    let days = days.clamp(1, 90);
    let span = format!("-{days} days");
    let mut sql = String::from(
        r#"
        SELECT
          COALESCE(NULLIF(TRIM(s.model), ''), 'unknown') AS model,
          COUNT(*) AS llm_calls,
          COALESCE(SUM(CAST(json_extract(e.payload_json, '$.input_tokens') AS INTEGER)), 0) AS input_tokens,
          COALESCE(SUM(CAST(json_extract(e.payload_json, '$.output_tokens') AS INTEGER)), 0) AS output_tokens
        FROM project_events e
        LEFT JOIN sessions s ON s.id = e.session_id
        WHERE e.event_type = 'llm_response_end'
          AND datetime(e.occurred_at) >= datetime('now', ?)
        "#,
    );
    if project_id.filter(|s| !s.is_empty()).is_some() {
        sql.push_str(" AND e.project_id = ?");
    }
    sql.push_str(" GROUP BY model ORDER BY input_tokens + output_tokens DESC LIMIT 50");
    let mut q = sqlx::query(&sql).bind(&span);
    if let Some(pid) = project_id.filter(|s| !s.is_empty()) {
        q = q.bind(pid);
    }
    let rows = q.fetch_all(db.pool()).await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let model: String = r.get("model");
            let input_tokens: i64 = r.get("input_tokens");
            let output_tokens: i64 = r.get("output_tokens");
            ModelUsageRow {
                provider: infer_provider(&model).into(),
                model: model.clone(),
                llm_calls: r.get("llm_calls"),
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
                estimated_cost_usd: estimate_model_cost_usd(&model, input_tokens, output_tokens),
            }
        })
        .collect())
}

/// Saved-hours KPI: compares completed session wall time vs manual baseline (V3).
pub async fn saved_hours_kpi(db: &DashboardDb, days: u32) -> Result<crate::schema::SavedHoursKpi> {
    use crate::schema::SavedHoursKpi;
    use sqlx::Row;
    let days = days.clamp(1, 90);
    let span = format!("-{days} days");
    let row = sqlx::query(
        r#"
        SELECT
          COUNT(*) AS sessions_completed,
          CAST(COALESCE(SUM(
            MAX(0.0, (julianday(COALESCE(NULLIF(TRIM(ended_at), ''), datetime('now')))
              - julianday(started_at)) * 24.0)
          ), 0) AS REAL) AS automation_hours
        FROM sessions
        WHERE status IN ('completed', 'done')
          AND started_at IS NOT NULL AND TRIM(started_at) != ''
          AND datetime(started_at) >= datetime('now', ?)
        "#,
    )
    .bind(&span)
    .fetch_one(db.pool())
    .await?;
    let sessions_completed: i64 = row.get("sessions_completed");
    let automation_hours: f64 = row.get::<f64, _>("automation_hours");
    let baseline_hours_per_session = baseline_session_hours();
    let estimated_manual_hours = sessions_completed as f64 * baseline_hours_per_session;
    let estimated_saved_hours = (estimated_manual_hours - automation_hours).max(0.0);
    let hourly_rate_usd = hourly_rate_usd();
    Ok(SavedHoursKpi {
        days,
        sessions_completed,
        automation_hours,
        baseline_hours_per_session,
        estimated_manual_hours,
        estimated_saved_hours,
        hourly_rate_usd,
        estimated_value_usd: estimated_saved_hours * hourly_rate_usd,
        generated_at: chrono::Utc::now().to_rfc3339(),
    })
}

#[must_use]
pub fn infer_provider(model: &str) -> &'static str {
    let m = model.to_ascii_lowercase();
    if m.contains("claude") || m.starts_with("anthropic") {
        "anthropic"
    } else if m.contains("gpt")
        || m.starts_with("o1")
        || m.starts_with("o3")
        || m.contains("openai")
    {
        "openai"
    } else if m.contains("gemini") || m.contains("google") {
        "google"
    } else if m.contains("deepseek") {
        "deepseek"
    } else if m.contains("glm") || m.contains("zhipu") {
        "z.ai"
    } else if m.contains("qwen") || m.contains("dashscope") {
        "alibaba"
    } else if m.contains("llama") || m.contains("meta") {
        "meta"
    } else if m == "unknown" || m.is_empty() {
        "unknown"
    } else {
        "other"
    }
}

fn model_token_rates(model: &str) -> (f64, f64) {
    let m = model.to_ascii_lowercase();
    if m.contains("opus") {
        (15.0, 75.0)
    } else if m.contains("sonnet") || m.contains("claude") {
        (3.0, 15.0)
    } else if m.contains("haiku") {
        (0.25, 1.25)
    } else if m.contains("gpt-4o-mini") || m.contains("mini") {
        (0.15, 0.6)
    } else if m.contains("gpt-4") || m.starts_with("o1") || m.starts_with("o3") {
        (2.5, 10.0)
    } else if m.contains("gemini") {
        (1.25, 5.0)
    } else if m.contains("deepseek") {
        (0.27, 1.1)
    } else {
        (
            std::env::var("ANYCODE_DASHBOARD_INPUT_USD_PER_M")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3.0),
            std::env::var("ANYCODE_DASHBOARD_OUTPUT_USD_PER_M")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(15.0),
        )
    }
}

fn estimate_model_cost_usd(model: &str, input_tokens: i64, output_tokens: i64) -> f64 {
    let (input_rate, output_rate) = model_token_rates(model);
    (input_tokens as f64 / 1_000_000.0) * input_rate
        + (output_tokens as f64 / 1_000_000.0) * output_rate
}

fn baseline_session_hours() -> f64 {
    std::env::var("ANYCODE_DASHBOARD_BASELINE_SESSION_MINUTES")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(45.0)
        / 60.0
}

fn hourly_rate_usd() -> f64 {
    std::env::var("ANYCODE_DASHBOARD_HOURLY_RATE_USD")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(50.0)
}

/// Rough USD estimate (override via env: `ANYCODE_DASHBOARD_INPUT_USD_PER_M`, `ANYCODE_DASHBOARD_OUTPUT_USD_PER_M`).
fn estimate_token_cost_usd(input_tokens: i64, output_tokens: i64) -> f64 {
    estimate_model_cost_usd("unknown", input_tokens, output_tokens)
}

/// Per-project LLM token usage.
pub async fn project_token_usage(
    db: &DashboardDb,
    project_id: &str,
    days: u32,
) -> Result<crate::schema::TokenUsageStats> {
    Ok(project_token_usage_detail(db, project_id, days)
        .await?
        .usage)
}

/// Per-session LLM token usage (all events for session).
pub async fn session_token_usage_detail(
    db: &DashboardDb,
    session_id: &str,
) -> Result<crate::schema::TokenUsageDetail> {
    use crate::schema::{TokenUsageDetail, TokenUsageStats};
    let by_model = usage_by_model_session(db, session_id).await?;
    let llm_calls: i64 = by_model.iter().map(|r| r.llm_calls).sum();
    let input_tokens: i64 = by_model.iter().map(|r| r.input_tokens).sum();
    let output_tokens: i64 = by_model.iter().map(|r| r.output_tokens).sum();
    let estimated_cost_usd: f64 = by_model.iter().map(|r| r.estimated_cost_usd).sum();
    Ok(TokenUsageDetail {
        usage: TokenUsageStats {
            days: 0,
            llm_calls,
            input_tokens,
            output_tokens,
            total_tokens: input_tokens + output_tokens,
            estimated_cost_usd,
            generated_at: chrono::Utc::now().to_rfc3339(),
        },
        by_model,
    })
}

async fn usage_by_model_session(
    db: &DashboardDb,
    session_id: &str,
) -> Result<Vec<crate::schema::ModelUsageRow>> {
    use crate::schema::ModelUsageRow;
    use sqlx::Row;
    let rows = sqlx::query(
        r#"
        SELECT
          COALESCE(NULLIF(TRIM(s.model), ''), 'unknown') AS model,
          COUNT(*) AS llm_calls,
          COALESCE(SUM(CAST(json_extract(e.payload_json, '$.input_tokens') AS INTEGER)), 0) AS input_tokens,
          COALESCE(SUM(CAST(json_extract(e.payload_json, '$.output_tokens') AS INTEGER)), 0) AS output_tokens
        FROM project_events e
        LEFT JOIN sessions s ON s.id = e.session_id
        WHERE e.event_type = 'llm_response_end'
          AND e.session_id = ?
        GROUP BY model ORDER BY input_tokens + output_tokens DESC LIMIT 20
        "#,
    )
    .bind(session_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| {
            let model: String = r.get("model");
            let input_tokens: i64 = r.get("input_tokens");
            let output_tokens: i64 = r.get("output_tokens");
            ModelUsageRow {
                provider: infer_provider(&model).into(),
                model: model.clone(),
                llm_calls: r.get("llm_calls"),
                input_tokens,
                output_tokens,
                total_tokens: input_tokens + output_tokens,
                estimated_cost_usd: estimate_model_cost_usd(&model, input_tokens, output_tokens),
            }
        })
        .collect())
}

/// CSV export for global or per-project usage (by project row).
pub async fn usage_export_csv(
    db: &DashboardDb,
    days: u32,
    project_id: Option<&str>,
) -> Result<String> {
    use sqlx::Row;
    let days = days.clamp(1, 90);
    let span = format!("-{days} days");
    let mut out = String::from("project_id,project_name,llm_calls,input_tokens,output_tokens,total_tokens,estimated_cost_usd\n");
    if let Some(pid) = project_id.filter(|s| !s.is_empty()) {
        let usage = project_token_usage(db, pid, days).await?;
        let name = db
            .get_project(pid)
            .await?
            .map(|p| p.name)
            .unwrap_or_else(|| pid.to_string());
        out.push_str(&format!(
            "{},{},{},{},{},{},{:.4}\n",
            pid,
            csv_escape(&name),
            usage.llm_calls,
            usage.input_tokens,
            usage.output_tokens,
            usage.total_tokens,
            usage.estimated_cost_usd
        ));
        return Ok(out);
    }
    let rows = sqlx::query(
        r#"
        SELECT
          e.project_id,
          p.name AS project_name,
          COUNT(*) AS llm_calls,
          COALESCE(SUM(CAST(json_extract(e.payload_json, '$.input_tokens') AS INTEGER)), 0) AS input_tokens,
          COALESCE(SUM(CAST(json_extract(e.payload_json, '$.output_tokens') AS INTEGER)), 0) AS output_tokens
        FROM project_events e
        LEFT JOIN projects p ON p.id = e.project_id
        WHERE e.event_type = 'llm_response_end'
          AND datetime(e.occurred_at) >= datetime('now', ?)
        GROUP BY e.project_id
        ORDER BY input_tokens + output_tokens DESC
        "#,
    )
    .bind(span)
    .fetch_all(db.pool())
    .await?;
    for r in rows {
        let input_tokens: i64 = r.get("input_tokens");
        let output_tokens: i64 = r.get("output_tokens");
        let total = input_tokens + output_tokens;
        let cost = estimate_token_cost_usd(input_tokens, output_tokens);
        out.push_str(&format!(
            "{},{},{},{},{},{},{:.4}\n",
            r.get::<String, _>("project_id"),
            csv_escape(
                &r.get::<Option<String>, _>("project_name")
                    .unwrap_or_default()
            ),
            r.get::<i64, _>("llm_calls"),
            input_tokens,
            output_tokens,
            total,
            cost
        ));
    }
    Ok(out)
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn blocked_alert_threshold() -> Option<i64> {
    std::env::var("ANYCODE_DASHBOARD_BLOCKED_ALERT_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|&t| t >= 0)
}

/// Emit at most one `blocked_threshold_exceeded` notification per hour when blocked sessions exceed threshold.
pub async fn maybe_emit_blocked_threshold_alert(db: &DashboardDb) -> Result<()> {
    let Some(threshold) = blocked_alert_threshold() else {
        return Ok(());
    };
    let stats = db.overview_stats().await?;
    if stats.sessions_blocked <= threshold {
        return Ok(());
    }
    let recent: i64 = sqlx::query_scalar(
        r#"
        SELECT COUNT(*) FROM auth_events
        WHERE event_type = 'blocked_threshold_exceeded'
          AND datetime(created_at) >= datetime('now', '-1 hour')
        "#,
    )
    .fetch_one(db.pool())
    .await?;
    if recent > 0 {
        return Ok(());
    }
    let detail = serde_json::json!({
        "title": "Blocked sessions exceeded threshold",
        "blocked_sessions": stats.sessions_blocked,
        "threshold": threshold,
    });
    crate::audit::record_audit(
        db,
        crate::audit::AuditEventInput {
            project_id: None,
            session_id: None,
            action: "blocked_threshold_exceeded".into(),
            risk: "medium".into(),
            detail: detail.clone(),
        },
    )
    .await?;
    crate::notifications::emit_local_log(db, None, None, "blocked_threshold_exceeded", detail)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CreateSessionRequest, UpsertProjectRequest};
    use tempfile::tempdir;

    #[test]
    fn token_cost_estimate() {
        assert!((super::estimate_token_cost_usd(1_000_000, 0) - 3.0).abs() < 0.01);
        assert!((super::estimate_token_cost_usd(0, 1_000_000) - 15.0).abs() < 0.01);
    }

    #[test]
    fn infer_provider_variants() {
        assert_eq!(infer_provider("claude-sonnet-4"), "anthropic");
        assert_eq!(infer_provider("gpt-4o"), "openai");
        assert_eq!(infer_provider("gemini-2.0-flash"), "google");
        assert_eq!(infer_provider("unknown"), "unknown");
        assert_eq!(infer_provider(""), "unknown");
    }

    #[test]
    fn model_cost_sonnet() {
        let cost = estimate_model_cost_usd("claude-sonnet-4", 1_000_000, 1_000_000);
        assert!((cost - 18.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn readiness_empty_db_ok() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("m.db")).await.unwrap();
        let r = global_readiness(&db).await.unwrap();
        assert_eq!(r.status, "ok");
    }

    #[tokio::test]
    async fn project_metrics_after_gate_failure() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("g.db")).await.unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/m".into(),
                name: Some("M".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "run".into(),
                task_id: None,
                title: "t".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.upsert_gate(
            &project.id,
            &session.id,
            "test",
            "test",
            "failed",
            true,
            "fail",
        )
        .await
        .unwrap();
        let m = project_metrics(&db, &project.id).await.unwrap();
        assert_eq!(m.failed_required_gates, 1);
        assert!(m.readiness_score < 100);
    }

    #[tokio::test]
    async fn global_timeline_empty_db() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("tl.db")).await.unwrap();
        let tl = global_timeline(&db, 7).await.unwrap();
        assert_eq!(tl.days, 7);
    }

    #[tokio::test]
    async fn blocked_threshold_alert_when_exceeded() {
        let dir = tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("blocked.db"))
            .await
            .unwrap();
        std::env::set_var("ANYCODE_DASHBOARD_BLOCKED_ALERT_THRESHOLD", "0");
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/blocked".into(),
                name: Some("B".into()),
                description: None,
                create_root: None,
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "run".into(),
                task_id: Some("t1".into()),
                title: "blocked".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        db.upsert_gate(
            &project.id,
            &session.id,
            "test gate",
            "echo fail",
            "failed",
            true,
            "failed",
        )
        .await
        .unwrap();
        db.refresh_session_trusted_status(&session.id)
            .await
            .unwrap();
        maybe_emit_blocked_threshold_alert(&db).await.unwrap();
        let recent = crate::audit::list_recent_notifications(&db, 5)
            .await
            .unwrap();
        assert!(recent
            .iter()
            .any(|n| n.action == "blocked_threshold_exceeded"));
        std::env::remove_var("ANYCODE_DASHBOARD_BLOCKED_ALERT_THRESHOLD");
    }
}
