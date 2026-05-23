use super::*;

pub async fn get_connector_github_issues(
    State(state): State<AppState>,
    Path(connector_id): Path<String>,
) -> impl IntoResponse {
    let (source_type, config) =
        match crate::notifications::get_connector_config(&state.db, &connector_id).await {
            Ok(Some(pair)) => pair,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "connector not found" })),
                )
                    .into_response()
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response()
            }
        };
    if source_type != "github" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "connector is not github type" })),
        )
            .into_response();
    }
    let repo = config
        .get("repo")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if repo.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "github connector missing repo in config" })),
        )
            .into_response();
    }
    let token = config
        .get("token")
        .and_then(|v| v.as_str())
        .filter(|s| *s != "***redacted***" && !s.is_empty())
        .map(str::to_string)
        .or_else(|| std::env::var("GITHUB_TOKEN").ok())
        .or_else(|| std::env::var("ANYCODE_GITHUB_TOKEN").ok());
    match crate::connectors::fetch_github_issues(repo, token.as_deref()).await {
        Ok(issues) => Json(json!({ "issues": issues, "repo": repo })).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_connector_linear_issues(
    State(state): State<AppState>,
    Path(connector_id): Path<String>,
) -> impl IntoResponse {
    let (source_type, config) =
        match crate::notifications::get_connector_config(&state.db, &connector_id).await {
            Ok(Some(pair)) => pair,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({ "error": "connector not found" })),
                )
                    .into_response()
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response()
            }
        };
    if source_type != "linear" {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "connector is not linear type" })),
        )
            .into_response();
    }
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
    let token = config
        .get("token")
        .and_then(|v| v.as_str())
        .filter(|s| *s != "***redacted***" && !s.is_empty())
        .map(str::to_string)
        .or_else(|| std::env::var("LINEAR_API_KEY").ok())
        .or_else(|| std::env::var("ANYCODE_LINEAR_API_KEY").ok())
        .unwrap_or_default();
    let team_label = team_key
        .map(str::to_string)
        .or_else(|| team_id.map(str::to_string))
        .unwrap_or_default();
    match crate::connectors::fetch_linear_issues(team_key, team_id, &token).await {
        Ok(issues) => Json(json!({ "issues": issues, "team": team_label })).into_response(),
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
