use super::*;

pub async fn list_artifacts(
    State(state): State<AppState>,
    Query(q): Query<ArtifactsQuery>,
) -> impl IntoResponse {
    match state
        .db
        .list_artifacts(
            q.project_id.as_deref(),
            q.session_id.as_deref(),
            q.kind.as_deref(),
            q.exclude_kind.as_deref(),
            q.trust_level.as_deref(),
            q.unverified_only,
            q.blocked_session_only,
            q.final_only,
            q.limit,
        )
        .await
    {
        Ok(artifacts) => Json(json!({ "artifacts": artifacts })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_session_artifacts(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Query(q): Query<ArtifactsQuery>,
) -> impl IntoResponse {
    match state
        .db
        .list_artifacts(
            None,
            Some(&session_id),
            q.kind.as_deref(),
            q.exclude_kind.as_deref(),
            q.trust_level.as_deref(),
            q.unverified_only,
            q.blocked_session_only,
            q.final_only,
            q.limit,
        )
        .await
    {
        Ok(artifacts) => Json(json!({ "artifacts": artifacts })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_project_artifacts(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Query(q): Query<ArtifactsQuery>,
) -> impl IntoResponse {
    match state
        .db
        .list_artifacts(
            Some(&project_id),
            q.session_id.as_deref(),
            q.kind.as_deref(),
            q.exclude_kind.as_deref(),
            q.trust_level.as_deref(),
            q.unverified_only,
            q.blocked_session_only,
            q.final_only,
            q.limit,
        )
        .await
    {
        Ok(artifacts) => Json(json!({ "artifacts": artifacts })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn reindex_project(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match state.db.get_project(&project_id).await {
        Ok(Some(p)) => {
            let _ = state
                .db
                .upsert_project(UpsertProjectRequest {
                    root_path: p.root_path.clone(),
                    name: Some(p.name),
                    description: Some(p.description),
                    create_root: None,
                    ..Default::default()
                })
                .await;
            let mut paths = state.workspace_paths.clone();
            if !paths.iter().any(|r| r == &p.root_path) {
                paths.push(p.root_path.clone());
            }
            let skills = crate::skills_scan::sync_skills_to_db(&state.db, &paths)
                .await
                .unwrap_or(0);
            let _ = crate::audit::record_audit(
                &state.db,
                crate::audit::AuditEventInput::low(
                    "project_reindex_requested",
                    json!({ "skills_synced": skills }),
                )
                .with_project(&project_id),
            )
            .await;
            Json(json!({
                "ok": true,
                "project_id": project_id,
                "skills_synced": skills,
            }))
            .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "project not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn index_project_assets(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match crate::asset_index::index_project_assets(&state.db, &project_id).await {
        Ok(result) => Json(json!({ "ok": true, "result": result })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn scan_session_artifacts(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> impl IntoResponse {
    let session = match state.db.get_session(&session_id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "session not found" })),
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
    let project = match state.db.get_project(&session.project_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "project not found" })),
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
    let root = std::path::Path::new(&project.root_path);
    if !root.is_dir() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "project root does not exist on disk" })),
        )
            .into_response();
    }
    let since = crate::workspace_scan::parse_session_started_at(&session.started_at);
    match crate::workspace_scan::scan_and_register_artifacts(
        &state.db,
        &session.project_id,
        &session_id,
        root,
        since,
    )
    .await
    {
        Ok(registered) => Json(json!({
            "ok": true,
            "session_id": session_id,
            "registered": registered,
        }))
        .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn get_artifact_detail(
    State(state): State<AppState>,
    Path(artifact_id): Path<String>,
) -> impl IntoResponse {
    match crate::asset_index::get_artifact_detail(&state.db, &artifact_id).await {
        Ok(Some(detail)) => Json(json!({ "artifact": detail })).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "artifact not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}
