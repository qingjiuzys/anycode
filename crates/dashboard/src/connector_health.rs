//! Connector reachability checks for doctor diagnostics (V3 Week 3).

use crate::db::DashboardDb;
use crate::schema::DoctorCheck;
use serde_json::Value;
use std::time::Duration;

const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

pub async fn connector_doctor_checks(db: &DashboardDb) -> Vec<DoctorCheck> {
    let Ok(connectors) = crate::notifications::list_connectors(db, None).await else {
        return Vec::new();
    };
    let mut checks = Vec::new();
    for c in connectors.into_iter().filter(|c| c.enabled) {
        let Some((source_type, config)) = crate::notifications::get_connector_config(db, &c.id)
            .await
            .ok()
            .flatten()
        else {
            continue;
        };
        let check = match source_type.as_str() {
            "github" => probe_github(&c.name, &config).await,
            "linear" => probe_linear(&c.name, &config).await,
            other => DoctorCheck {
                id: format!("connector_{}", c.id),
                status: "ok".into(),
                message: format!("Connector \"{name}\" ({other}) configured", name = c.name),
            },
        };
        checks.push(DoctorCheck {
            id: format!("connector_{}", c.id),
            ..check
        });
    }
    checks
}

async fn probe_github(name: &str, config: &Value) -> DoctorCheck {
    let repo = config
        .get("repo")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if repo.is_empty() {
        return DoctorCheck {
            id: String::new(),
            status: "warn".into(),
            message: format!("GitHub connector \"{name}\": missing repo in config"),
        };
    }
    let token = connector_token(config, "GITHUB_TOKEN", "ANYCODE_GITHUB_TOKEN");
    if token.is_none() {
        return DoctorCheck {
            id: String::new(),
            status: "warn".into(),
            message: format!(
                "GitHub connector \"{name}\": repo {repo} — set GITHUB_TOKEN to verify reachability"
            ),
        };
    }
    match tokio::time::timeout(
        PROBE_TIMEOUT,
        crate::connectors::fetch_github_issues(repo, token.as_deref()),
    )
    .await
    {
        Ok(Ok(_)) => DoctorCheck {
            id: String::new(),
            status: "ok".into(),
            message: format!("GitHub connector \"{name}\": {repo} reachable"),
        },
        Ok(Err(e)) => DoctorCheck {
            id: String::new(),
            status: "error".into(),
            message: format!("GitHub connector \"{name}\": {e}"),
        },
        Err(_) => DoctorCheck {
            id: String::new(),
            status: "error".into(),
            message: format!("GitHub connector \"{name}\": probe timed out"),
        },
    }
}

async fn probe_linear(name: &str, config: &Value) -> DoctorCheck {
    let team_key = config
        .get("team_key")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let team_id = config
        .get("team_id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if team_key.is_none() && team_id.is_none() {
        return DoctorCheck {
            id: String::new(),
            status: "warn".into(),
            message: format!("Linear connector \"{name}\": missing team_key or team_id"),
        };
    }
    let token = connector_token(config, "LINEAR_API_KEY", "ANYCODE_LINEAR_API_KEY");
    if token.as_deref().unwrap_or("").is_empty() {
        return DoctorCheck {
            id: String::new(),
            status: "warn".into(),
            message: format!(
                "Linear connector \"{name}\": set LINEAR_API_KEY to verify reachability"
            ),
        };
    }
    match tokio::time::timeout(
        PROBE_TIMEOUT,
        crate::connectors::fetch_linear_issues(team_key, team_id, token.as_deref().unwrap()),
    )
    .await
    {
        Ok(Ok(_)) => DoctorCheck {
            id: String::new(),
            status: "ok".into(),
            message: format!("Linear connector \"{name}\": team reachable"),
        },
        Ok(Err(e)) => DoctorCheck {
            id: String::new(),
            status: "error".into(),
            message: format!("Linear connector \"{name}\": {e}"),
        },
        Err(_) => DoctorCheck {
            id: String::new(),
            status: "error".into(),
            message: format!("Linear connector \"{name}\": probe timed out"),
        },
    }
}

fn connector_token(config: &Value, primary_env: &str, fallback_env: &str) -> Option<String> {
    config
        .get("token")
        .and_then(|v| v.as_str())
        .filter(|s| *s != "***redacted***" && !s.is_empty())
        .map(str::to_string)
        .or_else(|| std::env::var(primary_env).ok())
        .or_else(|| std::env::var(fallback_env).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_token_prefers_config() {
        let cfg = serde_json::json!({ "token": "cfg-tok" });
        assert_eq!(
            connector_token(&cfg, "LINEAR_API_KEY", "ANYCODE_LINEAR_API_KEY"),
            Some("cfg-tok".into())
        );
    }
}
