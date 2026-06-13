//! Linear GraphQL read-only preview (active issues).

use anyhow::{anyhow, Context, Result};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinearIssueSummary {
    pub identifier: String,
    pub title: String,
    pub state: String,
    pub url: String,
    pub updated_at: String,
    pub labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GraphQlResponse {
    data: Option<IssuesData>,
    errors: Option<Vec<GraphQlError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct IssuesData {
    issues: IssueConnection,
}

#[derive(Debug, Deserialize)]
struct IssueConnection {
    nodes: Vec<LinearIssueRow>,
}

#[derive(Debug, Deserialize)]
struct LinearIssueRow {
    identifier: String,
    title: String,
    url: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    state: LinearState,
    labels: LabelConnection,
}

#[derive(Debug, Deserialize)]
struct LinearState {
    name: String,
}

#[derive(Debug, Deserialize)]
struct LabelConnection {
    nodes: Vec<LinearLabel>,
}

#[derive(Debug, Deserialize)]
struct LinearLabel {
    name: String,
}

/// Fetch active issues for a Linear team (by `team_key` or `team_id`).
pub async fn fetch_linear_issues(
    team_key: Option<&str>,
    team_id: Option<&str>,
    token: &str,
) -> Result<Vec<LinearIssueSummary>> {
    let token = token.trim();
    if token.is_empty() {
        return Err(anyhow!("Linear API key required"));
    }
    let team_key = team_key.map(str::trim).filter(|s| !s.is_empty());
    let team_id = team_id.map(str::trim).filter(|s| !s.is_empty());
    if team_key.is_none() && team_id.is_none() {
        return Err(anyhow!("expected team_key or team_id in connector config"));
    }

    let team_filter = if let Some(key) = team_key {
        json!({ "key": { "eq": key } })
    } else {
        json!({ "id": { "eq": team_id.unwrap() } })
    };
    let body = json!({
        "query": r#"query TeamIssues($filter: IssueFilter, $first: Int!) {
  issues(filter: $filter, first: $first, orderBy: updatedAt) {
    nodes {
      identifier
      title
      url
      updatedAt
      state { name }
      labels { nodes { name } }
    }
  }
}"#,
        "variables": {
            "first": 20,
            "filter": {
                "team": team_filter,
                "state": { "type": { "nin": ["completed", "canceled"] } }
            }
        }
    });

    let client = reqwest::Client::builder()
        .user_agent(anycode_core::user_agent("anycode-dashboard"))
        .build()
        .context("http client")?;
    let res = client
        .post("https://api.linear.app/graphql")
        .header("Authorization", token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .context("linear request")?;
    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(anyhow!("Linear API {status}: {text}"));
    }
    let parsed: GraphQlResponse = res.json().await.context("linear json")?;
    if let Some(errors) = parsed.errors.filter(|e| !e.is_empty()) {
        return Err(anyhow!(
            "Linear GraphQL: {}",
            errors
                .into_iter()
                .map(|e| e.message)
                .collect::<Vec<_>>()
                .join("; ")
        ));
    }
    let rows = parsed
        .data
        .ok_or_else(|| anyhow!("Linear GraphQL: empty data"))?
        .issues
        .nodes;
    Ok(rows
        .into_iter()
        .map(|r| LinearIssueSummary {
            identifier: r.identifier,
            title: r.title,
            state: r.state.name,
            url: r.url,
            updated_at: r.updated_at,
            labels: r.labels.nodes.into_iter().map(|l| l.name).collect(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_missing_team_and_token() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(fetch_linear_issues(None, None, "tok"))
            .unwrap_err();
        assert!(err.to_string().contains("team_key"));
        let err = rt
            .block_on(fetch_linear_issues(Some("ENG"), None, ""))
            .unwrap_err();
        assert!(err.to_string().contains("API key"));
    }
}
