//! GitHub REST read-only preview (open issues).

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GithubIssueSummary {
    pub number: i64,
    pub title: String,
    pub state: String,
    pub html_url: String,
    pub updated_at: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GithubIssueRow {
    number: i64,
    title: String,
    state: String,
    html_url: String,
    updated_at: String,
    #[serde(default)]
    labels: Vec<GithubLabel>,
}

#[derive(Debug, Deserialize)]
struct GithubLabel {
    name: String,
}

/// Fetch open issues for `owner/repo`. Token optional (raises rate limit).
pub async fn fetch_github_issues(
    repo: &str,
    token: Option<&str>,
) -> Result<Vec<GithubIssueSummary>> {
    let repo = repo.trim().trim_start_matches("https://github.com/");
    if !repo.contains('/') || repo.starts_with('/') {
        return Err(anyhow!("expected owner/repo, got {repo:?}"));
    }
    let url = format!("https://api.github.com/repos/{repo}/issues?state=open&per_page=20");
    let client = reqwest::Client::builder()
        .user_agent("anycode-dashboard/1.0")
        .build()
        .context("http client")?;
    let mut req = client
        .get(&url)
        .header("Accept", "application/vnd.github+json");
    if let Some(t) = token.filter(|s| !s.is_empty()) {
        req = req.header("Authorization", format!("Bearer {t}"));
    }
    let res = req.send().await.context("github request")?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub API {status}: {body}"));
    }
    let rows: Vec<GithubIssueRow> = res.json().await.context("github json")?;
    Ok(rows
        .into_iter()
        .filter(|r| r.state == "open")
        .map(|r| GithubIssueSummary {
            number: r.number,
            title: r.title,
            state: r.state,
            html_url: r.html_url,
            updated_at: r.updated_at,
            labels: r.labels.into_iter().map(|l| l.name).collect(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_repo() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(fetch_github_issues("not-a-repo", None))
            .unwrap_err();
        assert!(err.to_string().contains("owner/repo"));
    }
}
