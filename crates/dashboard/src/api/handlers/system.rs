use super::*;

pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let account_api_url = std::env::var("ANYCODE_ACCOUNT_API_URL")
        .ok()
        .filter(|s| !s.trim().is_empty());
    let account_portal_url = std::env::var("ANYCODE_ACCOUNT_PORTAL_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| account_api_url.clone());
    Json(HealthResponse {
        ok: true,
        version: state.version.clone(),
        db_path: state.db.path().display().to_string(),
        mode: "local".into(),
        account_api_url,
        account_portal_url,
    })
}

pub async fn search_workbench(
    State(state): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> impl IntoResponse {
    match crate::search::search(&state.db, &q.q, q.limit).await {
        Ok(results) => Json(results).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_overview(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.overview_stats().await {
        Ok(stats) => Json(json!({ "overview": stats })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_bootstrap(State(state): State<AppState>) -> impl IntoResponse {
    match crate::bootstrap::bootstrap_summary(&state.db, &state.workspace_paths).await {
        Ok(summary) => Json(json!({ "bootstrap": summary })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
