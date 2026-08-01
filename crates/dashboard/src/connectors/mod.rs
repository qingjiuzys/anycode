//! Read-only external connector previews (GitHub / Linear).

pub mod github;
pub mod linear;

pub use github::{fetch_github_issues, normalize_github_repo, GithubIssueSummary};
pub use linear::{fetch_linear_issues, LinearIssueSummary};

use serde_json::Value;

/// Validate connector config at create/update time. Returns a user-facing error.
pub fn validate_connector_config(source_type: &str, config: &Value) -> Result<(), String> {
    match source_type.trim().to_lowercase().as_str() {
        "github" => {
            let repo = config
                .get("repo")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if repo.is_empty() {
                return Err("github connector requires config.repo (owner/repo)".into());
            }
            if normalize_github_repo(repo).is_none() {
                return Err(format!("expected owner/repo, got {repo:?}"));
            }
            Ok(())
        }
        "linear" => {
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
                return Err("linear connector requires config.team_key or config.team_id".into());
            }
            let config_token = ["token", "api_key"].iter().find_map(|k| {
                config
                    .get(*k)
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && *s != "***redacted***")
            });
            let env_token = std::env::var("LINEAR_API_KEY")
                .ok()
                .or_else(|| std::env::var("ANYCODE_LINEAR_API_KEY").ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            if config_token.is_none() && env_token.is_none() {
                return Err(
                    "Linear API key required (config.token / config.api_key, or LINEAR_API_KEY env)"
                        .into(),
                );
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn validates_github_repo_shape() {
        assert!(validate_connector_config("github", &json!({ "repo": "o/r" })).is_ok());
        assert!(validate_connector_config("github", &json!({ "repo": "not-a-repo" })).is_err());
        assert!(validate_connector_config("github", &json!({})).is_err());
    }

    #[test]
    fn validates_linear_team_and_key() {
        // No env / no token → reject.
        let prev = std::env::var("LINEAR_API_KEY").ok();
        std::env::remove_var("LINEAR_API_KEY");
        std::env::remove_var("ANYCODE_LINEAR_API_KEY");
        assert!(validate_connector_config("linear", &json!({ "team_key": "ENG" })).is_err());
        assert!(validate_connector_config(
            "linear",
            &json!({ "team_key": "ENG", "token": "lin_xxx" })
        )
        .is_ok());
        assert!(validate_connector_config("linear", &json!({ "token": "lin_xxx" })).is_err());
        match prev {
            Some(v) => std::env::set_var("LINEAR_API_KEY", v),
            None => std::env::remove_var("LINEAR_API_KEY"),
        }
    }
}
