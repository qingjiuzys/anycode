use super::*;

pub async fn list_project_gates(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    match state.db.list_gates_for_project(&project_id).await {
        Ok(gates) => Json(json!({ "gates": gates })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn list_gate_presets(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
) -> impl IntoResponse {
    let project = match state.db.get_project(&project_id).await {
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
    let presets = crate::gate_runner::list_presets(std::path::Path::new(&project.root_path));
    Json(json!({ "presets": presets })).into_response()
}

#[derive(Deserialize)]
pub struct ExecuteGateBody {
    pub preset_id: Option<String>,
    pub name: Option<String>,
    pub command: Option<String>,
    #[serde(default)]
    pub required: Option<bool>,
}

fn resolve_gate_command(
    body: &ExecuteGateBody,
    root: &std::path::Path,
) -> Result<(String, String), (StatusCode, String)> {
    if let Some(cmd) = body.command.as_ref().filter(|s| !s.trim().is_empty()) {
        let name = body
            .name
            .as_ref()
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| "custom".into());
        return Ok((name, cmd.clone()));
    }
    if let Some(preset_id) = body.preset_id.as_ref().filter(|s| !s.is_empty()) {
        let presets = crate::gate_runner::list_presets(root);
        if let Some(p) = presets.into_iter().find(|p| p.id == *preset_id) {
            return Ok((p.name, p.command));
        }
        return Err((StatusCode::BAD_REQUEST, "unknown preset_id".into()));
    }
    Err((
        StatusCode::BAD_REQUEST,
        "preset_id or command required".into(),
    ))
}

async fn persist_manual_gate_run(
    db: &crate::db::DashboardDb,
    project_id: &str,
    result: &crate::gate_runner::GateExecuteResult,
    required: bool,
) -> Result<String, anyhow::Error> {
    let session_id = db.ensure_manual_gate_session(project_id).await?;
    db.upsert_gate(
        project_id,
        &session_id,
        &result.name,
        &result.command,
        &result.status,
        required,
        &result.output_excerpt,
    )
    .await?;
    db.insert_event(crate::schema::InsertEventRequest {
        project_id: project_id.to_string(),
        session_id: Some(session_id.clone()),
        task_id: None,
        agent_id: None,
        event_type: "gate_executed".into(),
        severity: Some(if result.status == "passed" {
            "info".into()
        } else {
            "warn".into()
        }),
        title: format!("Gate {}: {}", result.name, result.status),
        body: Some(result.output_excerpt.clone()),
        payload: Some(json!({
            "name": result.name,
            "command": result.command,
            "status": result.status,
            "elapsed_ms": result.elapsed_ms,
            "required": required,
        })),
    })
    .await?;
    let _ = crate::audit::record_audit(
        db,
        crate::audit::AuditEventInput::low(
            "gate_executed",
            json!({
                "project_id": project_id,
                "name": result.name,
                "status": result.status,
                "elapsed_ms": result.elapsed_ms,
            }),
        )
        .with_project(project_id),
    )
    .await;
    Ok(session_id)
}

pub async fn execute_project_gate(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<ExecuteGateBody>,
) -> impl IntoResponse {
    let project = match state.db.get_project(&project_id).await {
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
            Json(json!({ "error": "project root path not found on disk" })),
        )
            .into_response();
    }
    let (name, command) = match resolve_gate_command(&body, root) {
        Ok(v) => v,
        Err((code, msg)) => {
            return (code, Json(json!({ "error": msg }))).into_response();
        }
    };
    match crate::gate_runner::execute_gate(root, &name, &command).await {
        Ok(result) => {
            let required = body.required.unwrap_or(false);
            if let Err(e) = persist_manual_gate_run(&state.db, &project_id, &result, required).await
            {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": e.to_string() })),
                )
                    .into_response();
            }
            Json(json!({ "result": result })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

pub async fn execute_project_gate_stream(
    State(state): State<AppState>,
    Path(project_id): Path<String>,
    Json(body): Json<ExecuteGateBody>,
) -> impl IntoResponse {
    let err_response = |msg: String| {
        let stream = stream! {
            yield Ok::<Event, Infallible>(Event::default().event("gate").data(
                json!({ "type": "error", "error": msg }).to_string(),
            ));
        };
        Sse::new(stream)
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
            .into_response()
    };

    let project = match state.db.get_project(&project_id).await {
        Ok(Some(p)) => p,
        Ok(None) => return err_response("project not found".into()),
        Err(e) => return err_response(e.to_string()),
    };
    let root_path = project.root_path.clone();
    let root = std::path::Path::new(&root_path);
    if !root.is_dir() {
        return err_response("project root path not found on disk".into());
    }
    let (name, command) = match resolve_gate_command(&body, root) {
        Ok(v) => v,
        Err((_code, msg)) => return err_response(msg),
    };
    let required = body.required.unwrap_or(false);
    let db = state.db.clone();
    let pid = project_id.clone();

    let stream = stream! {
        let (line_tx, mut line_rx) = tokio::sync::mpsc::channel(256);
        let (done_tx, done_rx) = tokio::sync::oneshot::channel();
        let root_owned = root_path.clone();
        let name = name.clone();
        let command = command.clone();
        tokio::spawn(async move {
            let result = crate::gate_runner::execute_gate_streaming(
                std::path::Path::new(&root_owned),
                &name,
                &command,
                line_tx,
            )
            .await;
            let _ = done_tx.send(result);
        });
        while let Some(line) = line_rx.recv().await {
            yield Ok::<Event, Infallible>(Event::default().event("gate").data(
                json!({ "type": "line", "line": line }).to_string(),
            ));
        }
        match done_rx.await {
            Ok(Ok(result)) => {
                if let Err(e) = persist_manual_gate_run(&db, &pid, &result, required).await {
                    yield Ok::<Event, Infallible>(Event::default().event("gate").data(
                        json!({ "type": "error", "error": e.to_string() }).to_string(),
                    ));
                } else {
                    yield Ok::<Event, Infallible>(Event::default().event("gate").data(
                        json!({ "type": "done", "result": result }).to_string(),
                    ));
                }
            }
            Ok(Err(e)) => {
                yield Ok::<Event, Infallible>(Event::default().event("gate").data(
                    json!({ "type": "error", "error": e.to_string() }).to_string(),
                ));
            }
            Err(_) => {}
        }
    };
    Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
        .into_response()
}
