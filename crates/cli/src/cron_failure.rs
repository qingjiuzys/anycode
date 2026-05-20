//! Cron failure routing for production automation (`failure_destination`).

use anycode_tools::CronJob;
use std::time::Duration;
use tracing::warn;

pub(crate) fn sanitize_failure_detail(detail: &str) -> String {
    detail.chars().take(500).collect()
}

pub(crate) async fn route_cron_failure_shell(job: &CronJob, detail: &str) {
    let Ok(shell) = std::env::var("ANYCODE_CRON_FAILURE_SHELL") else {
        warn!(
            target: "anycode_scheduler",
            job_id = %job.id,
            "failure_destination=shell but ANYCODE_CRON_FAILURE_SHELL is unset"
        );
        return;
    };
    let shell = shell.trim().to_string();
    if shell.is_empty() {
        return;
    }
    let job_id = job.id.clone();
    let session_id = job.session_id.clone().unwrap_or_default();
    let detail = detail.to_string();
    let result = tokio::task::spawn_blocking(move || {
        std::process::Command::new(&shell)
            .env("ANYCODE_CRON_JOB_ID", &job_id)
            .env("ANYCODE_CRON_SESSION_ID", &session_id)
            .env("ANYCODE_CRON_STATUS", "error")
            .env("ANYCODE_CRON_ERROR", &detail)
            .status()
    })
    .await;
    match result {
        Ok(Ok(status)) if status.success() => {}
        Ok(Ok(status)) => {
            warn!(
                target: "anycode_scheduler",
                job_id = %job.id,
                code = ?status.code(),
                "cron failure shell hook exited non-zero"
            );
        }
        Ok(Err(e)) => {
            warn!(
                target: "anycode_scheduler",
                job_id = %job.id,
                error = %e,
                "cron failure shell hook failed to start"
            );
        }
        Err(e) => {
            warn!(
                target: "anycode_scheduler",
                job_id = %job.id,
                error = %e,
                "cron failure shell hook task join failed"
            );
        }
    }
}

pub(crate) async fn route_cron_failure_http(job: &CronJob, detail: &str) {
    let Ok(url) = std::env::var("ANYCODE_CRON_FAILURE_WEBHOOK") else {
        warn!(
            target: "anycode_scheduler",
            job_id = %job.id,
            "failure_destination=http but ANYCODE_CRON_FAILURE_WEBHOOK is unset"
        );
        return;
    };
    let url = url.trim();
    if url.is_empty() {
        return;
    }
    let payload = serde_json::json!({
        "job_id": job.id,
        "session_id": job.session_id,
        "status": "error",
        "detail": detail,
    });
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            warn!(
                target: "anycode_scheduler",
                job_id = %job.id,
                error = %e,
                "failed to build HTTP client for cron failure webhook"
            );
            return;
        }
    };
    match client.post(url).json(&payload).send().await {
        Ok(resp) if resp.status().is_success() => {}
        Ok(resp) => {
            warn!(
                target: "anycode_scheduler",
                job_id = %job.id,
                status = %resp.status(),
                "cron failure webhook returned non-success status"
            );
        }
        Err(e) => {
            warn!(
                target: "anycode_scheduler",
                job_id = %job.id,
                error = %e,
                "cron failure webhook request failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::sanitize_failure_detail;

    #[test]
    fn sanitize_failure_detail_truncates_long_messages() {
        let long = "x".repeat(600);
        assert_eq!(sanitize_failure_detail(&long).len(), 500);
    }
}
