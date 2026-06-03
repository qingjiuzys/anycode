//! Dashboard service status and doctor diagnostics.

use crate::schema::{DoctorCheck, DoctorReport, ServiceStatusDetail};
use anycode_llm::config_models::ModelFallbackConfig;
use anycode_llm::{normalize_provider_id, read_config_value, read_model_fallback, string_field};
use std::net::{SocketAddr, TcpListener as StdTcpListener};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

fn fallback_configured(fb: &ModelFallbackConfig) -> bool {
    fb.provider.as_ref().is_some_and(|s| !s.trim().is_empty())
        && fb.model.as_ref().is_some_and(|s| !s.trim().is_empty())
}

/// LLM / `~/.anycode/config.json` checks (safe to call without network).
pub fn llm_doctor_checks() -> Vec<DoctorCheck> {
    let mut checks = Vec::new();
    let (path, cfg) = match read_config_value(None) {
        Ok(v) => v,
        Err(e) => {
            checks.push(DoctorCheck {
                id: "llm_config_read".into(),
                status: "error".into(),
                message: format!("Failed to read config: {e}"),
            });
            return checks;
        }
    };

    let exists = path.is_file();
    checks.push(DoctorCheck {
        id: "llm_config_exists".into(),
        status: if exists { "ok" } else { "warn" }.into(),
        message: if exists {
            format!("config.json found at {}", path.display())
        } else {
            format!(
                "config.json not found at {} — run `anycode model` or create the file",
                path.display()
            )
        },
    });

    let api_key = string_field(&cfg, "api_key", "api_key");
    checks.push(DoctorCheck {
        id: "llm_api_key".into(),
        status: if api_key.is_some() { "ok" } else { "warn" }.into(),
        message: if api_key.is_some() {
            "Primary api_key is configured".into()
        } else {
            "api_key not set in config.json".into()
        },
    });

    let provider_raw = string_field(&cfg, "provider", "provider").unwrap_or_default();
    let norm = normalize_provider_id(&provider_raw);
    if norm == "google" && !fallback_configured(&read_model_fallback(&cfg)) {
        checks.push(DoctorCheck {
            id: "llm_google_fallback".into(),
            status: "warn".into(),
            message: "Google provider has no model_fallback — geo/rate-limit failover recommended"
                .into(),
        });
    }

    checks
}

pub fn is_loopback_host(host: &str) -> bool {
    host == "127.0.0.1" || host == "localhost" || host == "::1"
}

pub fn build_service_status(
    host: &str,
    port: u16,
    version: &str,
    db_path: &Path,
    static_dir: Option<&Path>,
    started_at: &str,
    pid: u32,
    sse_subscribers: usize,
    last_event_at: Option<&str>,
) -> ServiceStatusDetail {
    let ui_dist = static_dir.map(|p| p.display().to_string());
    let ui_dist_present = static_dir.is_some_and(|p| p.join("index.html").is_file());
    ServiceStatusDetail {
        name: "dashboard".into(),
        host: host.into(),
        port,
        status: "running".into(),
        auth_mode: if is_loopback_host(host) {
            "local_trusted".into()
        } else {
            "token_required".into()
        },
        version: version.into(),
        pid: Some(pid),
        started_at: started_at.into(),
        db_path: db_path.display().to_string(),
        ui_dist,
        ui_dist_present,
        sse_subscribers,
        last_event_at: last_event_at.map(str::to_string),
        loopback: is_loopback_host(host),
    }
}

pub fn doctor_overall_status(checks: &[DoctorCheck]) -> &'static str {
    if checks.iter().any(|c| c.status == "error") {
        "error"
    } else if checks.iter().any(|c| c.status == "warn") {
        "warn"
    } else {
        "ok"
    }
}

pub fn run_doctor_checks(
    host: &str,
    port: u16,
    db_path: &Path,
    static_dir: Option<&Path>,
) -> DoctorReport {
    let mut checks = Vec::new();

    let db_exists = db_path.is_file();
    checks.push(DoctorCheck {
        id: "db_exists".into(),
        status: if db_exists { "ok" } else { "warn" }.into(),
        message: if db_exists {
            format!("Database found at {}", db_path.display())
        } else {
            format!(
                "Database not found at {} (will be created on first run)",
                db_path.display()
            )
        },
    });

    if db_exists {
        let writable = db_path
            .parent()
            .map(|p| {
                p.metadata()
                    .map(|m| !m.permissions().readonly())
                    .unwrap_or(false)
            })
            .unwrap_or(false);
        checks.push(DoctorCheck {
            id: "db_writable".into(),
            status: if writable { "ok" } else { "error" }.into(),
            message: if writable {
                "Database directory is writable".into()
            } else {
                "Database directory is not writable".into()
            },
        });
    }

    let ui_present = static_dir.is_some_and(|d| d.join("index.html").is_file());
    checks.push(DoctorCheck {
        id: "ui_dist".into(),
        status: if ui_present { "ok" } else { "warn" }.into(),
        message: if ui_present {
            format!("UI dist found: {}", static_dir.unwrap().display())
        } else {
            "UI dist missing — run ./scripts/build-dashboard-ui.sh".into()
        },
    });

    let loopback = is_loopback_host(host);
    checks.push(DoctorCheck {
        id: "loopback_binding".into(),
        status: if loopback { "ok" } else { "warn" }.into(),
        message: if loopback {
            format!("Bound to loopback {host}:{port}")
        } else {
            format!("Non-loopback binding {host}:{port} — API token required")
        },
    });

    let trigger_ok = crate::task_trigger::triggers_enabled()
        && (loopback
            || std::env::var("ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE")
                .ok()
                .is_some_and(|v| v == "1"));
    checks.push(DoctorCheck {
        id: "ui_trigger_run".into(),
        status: if !crate::task_trigger::triggers_enabled() {
            "warn"
        } else if trigger_ok {
            "ok"
        } else {
            "warn"
        }
        .into(),
        message: if !crate::task_trigger::triggers_enabled() {
            "UI trigger run disabled (ANYCODE_DASHBOARD_TRIGGER_RUN=0)".into()
        } else if trigger_ok {
            "UI trigger run allowed for this binding".into()
        } else {
            "UI trigger run blocked on non-loopback — set ANYCODE_DASHBOARD_TRIGGER_RUN_REMOTE=1 to override".into()
        },
    });

    let approval_ok = crate::approval_ipc::web_approvals_enabled()
        && (loopback
            || std::env::var("ANYCODE_DASHBOARD_WEB_APPROVAL_REMOTE")
                .ok()
                .is_some_and(|v| v == "1"));
    checks.push(DoctorCheck {
        id: "ui_web_approval".into(),
        status: if !crate::approval_ipc::web_approvals_enabled() {
            "warn"
        } else if approval_ok {
            "ok"
        } else {
            "warn"
        }
        .into(),
        message: if !crate::approval_ipc::web_approvals_enabled() {
            "Web tool approval disabled (ANYCODE_DASHBOARD_WEB_APPROVAL=0)".into()
        } else if approval_ok {
            "Web tool approval respond allowed for this binding".into()
        } else {
            "Web approval respond blocked on non-loopback — set ANYCODE_DASHBOARD_WEB_APPROVAL_REMOTE=1".into()
        },
    });

    let port_free = port_available(host, port);
    checks.push(DoctorCheck {
        id: "port_available".into(),
        status: if port_free { "ok" } else { "warn" }.into(),
        message: if port_free {
            format!("Port {port} is available")
        } else {
            format!("Port {port} is already in use (this is expected when doctor runs from the live dashboard)")
        },
    });

    let swept =
        crate::approval_ipc::sweep_stale_pending(crate::approval_ipc::STALE_PENDING_MAX_AGE_SECS);
    let swept_active = crate::cancel_ipc::sweep_stale_active();
    let pending = crate::approval_ipc::pending_summary().pending_total;
    checks.push(DoctorCheck {
        id: "approval_ipc_pending".into(),
        status: if pending == 0 { "ok" } else { "warn" }.into(),
        message: if pending == 0 {
            "No pending Web tool approvals on disk".into()
        } else {
            format!("{pending} pending approval(s) on disk (swept {swept} stale this check)")
        },
    });

    checks.push(DoctorCheck {
        id: "cancel_ipc_active".into(),
        status: "ok".into(),
        message: format!("Swept {swept_active} stale active session registration(s)"),
    });

    let mcp_strict = std::env::var("ANYCODE_MCP_STRICT").ok().is_some();
    let mcp_quota = std::env::var("ANYCODE_MCP_MAX_CALLS_PER_SERVER")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0);
    checks.push(DoctorCheck {
        id: "mcp_governance".into(),
        status: if mcp_strict || mcp_quota.is_some() {
            "ok"
        } else {
            "warn"
        }
        .into(),
        message: if mcp_strict {
            format!(
                "MCP strict whitelist active{}",
                mcp_quota
                    .map(|q| format!("; per-server quota={q}"))
                    .unwrap_or_default()
            )
        } else if let Some(q) = mcp_quota {
            format!("MCP per-server call quota={q} (strict whitelist off)")
        } else {
            "MCP governance env unset (ANYCODE_MCP_STRICT / ANYCODE_MCP_MAX_CALLS_PER_SERVER)"
                .into()
        },
    });

    let overall = doctor_overall_status(&checks);

    DoctorReport {
        status: overall.into(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        checks,
        next_steps: Vec::new(),
    }
}

pub fn doctor_next_steps(
    report: &DoctorReport,
    has_projects: bool,
    active_tokens: i64,
    loopback: bool,
) -> Vec<String> {
    let mut steps = Vec::new();
    if report
        .checks
        .iter()
        .any(|c| c.id == "ui_dist" && c.status == "warn")
    {
        steps.push("Run ./scripts/build-dashboard-ui.sh to build the UI".into());
    }
    if !has_projects {
        steps.push("Run `anycode run` or `anycode goal` in a project directory".into());
    }
    if !loopback && active_tokens == 0 {
        steps.push("Create an API token: `anycode dashboard token create`".into());
    }
    if report
        .checks
        .iter()
        .any(|c| c.id == "skills_starter_pack" && c.status == "warn")
    {
        steps.push(
            "Install office starter skills: `anycode skills install-starter` or Agents page button"
                .into(),
        );
    }
    if report
        .checks
        .iter()
        .any(|c| c.id == "knowledge_index" && c.status == "warn")
    {
        steps.push("Reindex project knowledge in Settings → Project knowledge".into());
    }
    if report
        .checks
        .iter()
        .any(|c| c.id == "port_available" && c.status == "warn")
    {
        steps.push("If the dashboard is not already running, free the dashboard port or use `--port` with another value".into());
    }
    if steps.is_empty() && report.status == "ok" {
        steps.push("Start dashboard: `anycode dashboard --open`".into());
    }
    if report.status == "ok" {
        steps.push("Digital Workbench status: docs/digital-workbench-STATUS.md".into());
    }
    if report
        .checks
        .iter()
        .any(|c| c.id.starts_with("connector_") && c.status == "warn")
    {
        steps.push(
            "Set connector API tokens (GITHUB_TOKEN / LINEAR_API_KEY) to verify reachability"
                .into(),
        );
    }
    if report
        .checks
        .iter()
        .any(|c| c.id == "llm_config_exists" && c.status == "warn")
    {
        steps.push("Configure LLM: `anycode model` or Dashboard Settings → Models".into());
    }
    if report
        .checks
        .iter()
        .any(|c| c.id == "llm_api_key" && c.status == "warn")
    {
        steps.push("Set `api_key` in ~/.anycode/config.json (Dashboard Settings → Models)".into());
    }
    if report
        .checks
        .iter()
        .any(|c| c.id == "llm_google_fallback" && c.status == "warn")
    {
        steps.push(
            "Add runtime.model_fallback provider+model for Google geo/rate-limit failover".into(),
        );
    }
    steps
}

fn port_available(host: &str, port: u16) -> bool {
    let addr: SocketAddr = match format!("{host}:{port}").parse() {
        Ok(a) => a,
        Err(_) => return false,
    };
    StdTcpListener::bind(addr).is_ok()
}

pub fn dist_build_time(static_dir: &Path) -> Option<String> {
    let index = static_dir.join("index.html");
    let meta = std::fs::metadata(index).ok()?;
    let modified = meta.modified().ok()?;
    let secs = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(format!("{secs}"))
}

pub fn suggest_backup_path(db_path: &Path) -> PathBuf {
    let stem = db_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("projects");
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    db_path.with_file_name(format!("{stem}.backup.{ts}.db"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    #[test]
    fn loopback_detection() {
        assert!(is_loopback_host("127.0.0.1"));
        assert!(is_loopback_host("localhost"));
        assert!(!is_loopback_host("0.0.0.0"));
    }

    #[test]
    fn llm_doctor_warns_google_without_fallback() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".anycode");
        std::fs::create_dir_all(&path).unwrap();
        let cfg_path = path.join("config.json");
        let mut f = std::fs::File::create(&cfg_path).unwrap();
        write!(
            f,
            "{}",
            json!({
                "provider": "google",
                "model": "gemini-2.0-flash",
                "api_key": "test-key"
            })
        )
        .unwrap();
        std::env::set_var("HOME", dir.path());
        let checks = llm_doctor_checks();
        assert!(checks
            .iter()
            .any(|c| c.id == "llm_google_fallback" && c.status == "warn"));
    }
}
