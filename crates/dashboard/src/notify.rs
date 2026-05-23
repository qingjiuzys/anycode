//! Push events to a running dashboard server for live SSE (DB already written by CLI).

use crate::schema::{InsertEventRequest, ProjectEvent};
use std::time::Duration;

const DEFAULT_BASE: &str = "http://127.0.0.1:43180";

/// Whether to POST publish notifications after local SQLite insert.
#[must_use]
pub fn notify_enabled() -> bool {
    !matches!(
        std::env::var("ANYCODE_DASHBOARD_NOTIFY").as_deref(),
        Ok("0") | Ok("false") | Ok("off")
    )
}

fn base_url() -> String {
    std::env::var("ANYCODE_DASHBOARD_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_BASE.to_string())
}

/// Fire-and-forget: publish an already-persisted event to the dashboard SSE bus.
pub fn spawn_publish_event(event: ProjectEvent) {
    if !notify_enabled() {
        return;
    }
    tokio::spawn(async move {
        if let Err(e) = publish_event_http(&event).await {
            tracing::debug!(error = %e, "dashboard SSE notify skipped");
        }
    });
}

async fn publish_event_http(event: &ProjectEvent) -> anyhow::Result<()> {
    let url = format!(
        "{}/api/projects/{}/events/publish",
        base_url().trim_end_matches('/'),
        event.project_id
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    let res = client.post(&url).json(event).send().await?;
    if !res.status().is_success() {
        anyhow::bail!("dashboard publish HTTP {}", res.status());
    }
    Ok(())
}

/// Publish via HTTP only (no local DB); used when API is the sole writer.
#[allow(dead_code)]
pub async fn post_insert_event(
    project_id: &str,
    req: InsertEventRequest,
) -> anyhow::Result<ProjectEvent> {
    let url = format!(
        "{}/api/projects/{}/events",
        base_url().trim_end_matches('/'),
        project_id
    );
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let res = client.post(&url).json(&req).send().await?;
    if !res.status().is_success() {
        anyhow::bail!("dashboard insert HTTP {}", res.status());
    }
    let body: serde_json::Value = res.json().await?;
    let evt = body
        .get("event")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing event in response"))?;
    Ok(serde_json::from_value(evt)?)
}
