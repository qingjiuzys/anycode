use super::*;
use crate::managed_local_llm::MINICPM5_PRESET_ID;

pub async fn get_managed_local_status(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.managed_local_llm.legacy_status().await)
}

pub async fn post_managed_local_download(State(state): State<AppState>) -> impl IntoResponse {
    local_download(&state, MINICPM5_PRESET_ID).await
}

pub async fn list_local_models(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({ "models": state.managed_local_llm.list_status().await }))
}

pub async fn get_local_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    match state.managed_local_llm.status(&model_id).await {
        Ok(status) => Json(status).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_local_model_download(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    local_download(&state, &model_id).await
}

async fn local_download(state: &AppState, model_id: &str) -> axum::response::Response {
    match state.managed_local_llm.start_download(model_id).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_managed_local_cancel_download(
    State(state): State<AppState>,
) -> impl IntoResponse {
    local_cancel_download(&state, MINICPM5_PRESET_ID).await
}

pub async fn post_local_model_cancel_download(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    local_cancel_download(&state, &model_id).await
}

async fn local_cancel_download(state: &AppState, model_id: &str) -> axum::response::Response {
    match state.managed_local_llm.cancel_download(model_id).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(error) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "ok": false, "error": error.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_managed_local_start(State(state): State<AppState>) -> impl IntoResponse {
    local_start(&state, MINICPM5_PRESET_ID).await
}

pub async fn post_local_model_start(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    local_start(&state, &model_id).await
}

async fn local_start(state: &AppState, model_id: &str) -> axum::response::Response {
    match state.managed_local_llm.start(model_id).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn post_managed_local_stop(State(state): State<AppState>) -> impl IntoResponse {
    local_stop(&state, MINICPM5_PRESET_ID).await
}

pub async fn post_local_model_stop(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    local_stop(&state, &model_id).await
}

async fn local_stop(state: &AppState, model_id: &str) -> axum::response::Response {
    match state.managed_local_llm.stop(model_id).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn delete_managed_local_model(State(state): State<AppState>) -> impl IntoResponse {
    local_delete(&state, MINICPM5_PRESET_ID).await
}

pub async fn delete_local_model(
    State(state): State<AppState>,
    Path(model_id): Path<String>,
) -> impl IntoResponse {
    local_delete(&state, &model_id).await
}

async fn local_delete(state: &AppState, model_id: &str) -> axum::response::Response {
    match state.managed_local_llm.delete_model(model_id).await {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": e.to_string() })),
        )
            .into_response(),
    }
}
