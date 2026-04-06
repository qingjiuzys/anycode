//! Daemon 最小 HTTP 接口：与 CLI `run` 共用同一 `AgentRuntime`（仅建议绑定 127.0.0.1）。

use crate::app_config::Config;
use crate::tasks::execute_workflow_runtime;
use crate::workspace;
use anycode_agent::AgentRuntime;
use anycode_channels::profile_for_channel_type;
use anycode_core::prelude::*;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, HeaderMap, Method, Request, Response, Server, StatusCode};
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, VecDeque};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use uuid::Uuid;

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_TASK_HISTORY: usize = 200;

fn optional_daemon_token() -> Option<String> {
    static TOKEN: OnceLock<Option<String>> = OnceLock::new();
    TOKEN
        .get_or_init(|| {
            std::env::var("ANYCODE_DAEMON_TOKEN")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .clone()
}

fn daemon_auth_ok(headers: &HeaderMap, expected: &str) -> bool {
    let bearer = headers
        .get(hyper::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer ").map(str::trim))
        .map(|t| t == expected)
        .unwrap_or(false);
    let header_tok = headers
        .get("x-anycode-token")
        .and_then(|h| h.to_str().ok())
        .map(|t| t == expected)
        .unwrap_or(false);
    bearer || header_tok
}

#[derive(Debug, Deserialize)]
struct PostTaskBody {
    #[serde(default)]
    agent: String,
    prompt: String,
    #[serde(default)]
    working_directory: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    workflow: Option<String>,
    #[serde(default)]
    goal: Option<String>,
}

#[derive(Debug, serde::Serialize)]
struct PostTaskResponse {
    task_id: String,
    result: TaskResult,
    output_log_path: String,
}

#[derive(Debug, serde::Serialize, Clone)]
struct TaskHistoryEntry {
    task_id: String,
    session_id: String,
    working_directory: String,
    agent: String,
    mode: Option<String>,
    workflow: Option<String>,
    goal: Option<String>,
    prompt_preview: String,
    result_ok: bool,
    error: Option<String>,
    created_at: String,
}

struct DaemonState {
    runtime: Arc<AgentRuntime>,
    config: Arc<Config>,
    history: Mutex<VecDeque<TaskHistoryEntry>>,
}

fn prompt_preview(s: &str) -> String {
    let t = s.trim();
    if t.chars().count() <= 200 {
        t.to_string()
    } else {
        format!("{}…", t.chars().take(200).collect::<String>())
    }
}

fn merged_config_for_cwd(base: &Arc<Config>, working_dir: &std::path::Path) -> Config {
    let mut cfg = (**base).clone();
    workspace::apply_project_overlays(&mut cfg, working_dir);
    cfg
}

async fn handle(
    req: Request<Body>,
    state: Arc<DaemonState>,
) -> Result<Response<Body>, Infallible> {
    if req.method() == &Method::POST && req.uri().path() == "/v1/tasks" {
        if let Some(ref tok) = optional_daemon_token() {
            if !daemon_auth_ok(req.headers(), tok) {
                return Ok(json_response(
                    StatusCode::UNAUTHORIZED,
                    json!({
                        "error": "unauthorized",
                        "hint": "set Authorization: Bearer <ANYCODE_DAEMON_TOKEN> or header x-anycode-token"
                    }),
                ));
            }
        }
    }

    let path = req.uri().path().to_string();

    match (req.method(), path.as_str()) {
        (&Method::GET, "/health") => Ok(Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "text/plain; charset=utf-8")
            .body(Body::from("ok"))
            .unwrap()),
        (&Method::GET, "/status") => Ok(json_response(
            StatusCode::OK,
            json!({
                "ok": true,
                "current_workspace": std::env::current_dir().ok().map(|p| p.display().to_string()),
                "recent_projects": workspace::recent_projects(5),
                "daemon_token_enabled": optional_daemon_token().is_some(),
            }),
        )),
        (&Method::GET, "/v1/workspace") => Ok(json_response(
            StatusCode::OK,
            json!({
                "current_workspace": std::env::current_dir().ok().map(|p| p.display().to_string()),
                "projects": workspace::list_projects(50),
            }),
        )),
        (&Method::GET, "/v1/runtime/profile") => {
            let merged = if let Ok(cwd) = std::env::current_dir() {
                let wd = std::fs::canonicalize(&cwd).unwrap_or(cwd);
                merged_config_for_cwd(&state.config, &wd)
            } else {
                (*state.config).clone()
            };
            let web = profile_for_channel_type(&ChannelType::Web);
            Ok(json_response(
                StatusCode::OK,
                json!({
                    "channel": web.id,
                    "default_mode": merged.runtime.default_mode.as_str(),
                    "default_agent": merged.runtime.default_mode.default_agent().as_str(),
                    "workspace_label": merged.runtime.workspace_project_label,
                    "workspace_channel_profile": merged.runtime.workspace_channel_profile,
                }),
            ))
        }
        (&Method::GET, "/v1/tasks") => {
            let query = req.uri().query().unwrap_or("");
            let mut limit = 50usize;
            let mut session_filter: Option<String> = None;
            for pair in query.split('&') {
                if pair.is_empty() {
                    continue;
                }
                let mut it = pair.splitn(2, '=');
                let k = it.next().unwrap_or("");
                let v = it.next().unwrap_or("");
                match k {
                    "limit" => {
                        if let Ok(n) = v.parse::<usize>() {
                            limit = n.min(200);
                        }
                    }
                    "session_id" => {
                        if !v.is_empty() {
                            session_filter = Some(v.to_string());
                        }
                    }
                    _ => {}
                }
            }
            let hist = state.history.lock().unwrap();
            let list: Vec<TaskHistoryEntry> = hist
                .iter()
                .filter(|e| {
                    session_filter
                        .as_ref()
                        .map(|s| s == &e.session_id)
                        .unwrap_or(true)
                })
                .take(limit)
                .cloned()
                .collect();
            Ok(json_response(StatusCode::OK, json!({ "tasks": list })))
        }
        _ if req.method() == &Method::GET && path.starts_with("/v1/tasks/") => {
            let id = path.trim_start_matches("/v1/tasks/");
            if id.is_empty() || id.contains('/') {
                return Ok(Response::builder()
                    .status(StatusCode::NOT_FOUND)
                    .header("content-type", "application/json; charset=utf-8")
                    .body(Body::from(r#"{"error":"not_found"}"#))
                    .unwrap());
            }
            let hist = state.history.lock().unwrap();
            let found = hist.iter().find(|e| e.task_id == id);
            match found {
                Some(e) => Ok(json_response(StatusCode::OK, json!(e))),
                None => Ok(json_response(
                    StatusCode::NOT_FOUND,
                    json!({"error":"not_found","task_id":id}),
                )),
            }
        }

        (&Method::POST, "/v1/tasks") => Ok(handle_post_task(req, state).await),

        _ => Ok(Response::builder()
            .status(StatusCode::NOT_FOUND)
            .header("content-type", "application/json; charset=utf-8")
            .body(Body::from(r#"{"error":"not_found"}"#))
            .unwrap()),
    }
}

fn push_history(state: &DaemonState, entry: TaskHistoryEntry) {
    let mut g = state.history.lock().unwrap();
    g.push_front(entry);
    while g.len() > MAX_TASK_HISTORY {
        g.pop_back();
    }
}

async fn handle_post_task(req: Request<Body>, state: Arc<DaemonState>) -> Response<Body> {
    let body = match hyper::body::to_bytes(req.into_body()).await {
        Ok(b) => b,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                json!({ "error": format!("read body: {}", e) }),
            );
        }
    };
    if body.len() > MAX_BODY_BYTES {
        return json_response(
            StatusCode::PAYLOAD_TOO_LARGE,
            json!({ "error": "body too large", "max_bytes": MAX_BODY_BYTES }),
        );
    }

    let parsed: PostTaskBody = match serde_json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => {
            return json_response(
                StatusCode::BAD_REQUEST,
                json!({ "error": format!("invalid json: {}", e) }),
            );
        }
    };

    if parsed.prompt.trim().is_empty() {
        return json_response(
            StatusCode::BAD_REQUEST,
            json!({ "error": "prompt is required" }),
        );
    }

    let mut working_dir = match parsed.working_directory {
        Some(p) if !p.trim().is_empty() => std::path::PathBuf::from(p.trim()),
        _ => match std::env::current_dir() {
            Ok(p) => p,
            Err(e) => {
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({ "error": format!("current_dir: {}", e) }),
                );
            }
        },
    };
    working_dir = std::fs::canonicalize(&working_dir).unwrap_or(working_dir);

    let merged = merged_config_for_cwd(&state.config, &working_dir);

    let runtime_mode = parsed
        .mode
        .as_deref()
        .and_then(RuntimeMode::parse)
        .unwrap_or(merged.runtime.default_mode);
    let resolved_agent = if parsed.agent.trim().is_empty() {
        runtime_mode.default_agent().as_str().to_string()
    } else {
        parsed.agent.trim().to_string()
    };
    let agent_for_history = resolved_agent.clone();
    let prompt_preview_text = prompt_preview(&parsed.prompt);
    let workflow_path = parsed.workflow.clone();
    let goal_spec = parsed.goal.clone().map(|goal| GoalSpec {
        objective: goal.clone(),
        done_when: None,
        allow_infinite_retries: true,
    });

    let task = Task {
        id: Uuid::new_v4(),
        agent_type: AgentType::new(resolved_agent),
        prompt: parsed.prompt,
        context: TaskContext {
            session_id: Uuid::new_v4(),
            working_directory: working_dir.to_string_lossy().to_string(),
            environment: HashMap::new(),
            user_id: None,
            system_prompt_append: Some(format!(
                "## Channel Runtime\nchannel=web\nruntime_mode={}\nworkflow={}\nworkspace_channel_profile={:?}\nworkspace_label={:?}\nFor web channel requests, prefer workspace-assistant behavior unless the selected mode or explicit agent requires coding.",
                runtime_mode.as_str(),
                workflow_path.clone().unwrap_or_else(|| "(none)".to_string()),
                merged.runtime.workspace_channel_profile,
                merged.runtime.workspace_project_label,
            )),
        },
        created_at: chrono::Utc::now(),
    };

    let task_id = task.id;
    let session_id = task.context.session_id;
    let log_path = match DiskTaskOutput::new_default() {
        Ok(d) => d.output_path(task_id),
        Err(e) => {
            return json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({ "error": format!("disk output: {}", e) }),
            );
        }
    };

    let created_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let result = if let Some(workflow_path) = workflow_path.as_deref() {
        let wf_path = {
            let raw = std::path::PathBuf::from(workflow_path);
            if raw.is_absolute() {
                raw
            } else {
                working_dir.join(raw)
            }
        };
        match execute_workflow_runtime(&state.runtime, &working_dir, &wf_path, Some(task.prompt.clone()))
            .await
        {
            Ok(r) => r,
            Err(e) => {
                push_history(
                    &state,
                    TaskHistoryEntry {
                        task_id: task_id.to_string(),
                        session_id: session_id.to_string(),
                        working_directory: working_dir.to_string_lossy().to_string(),
                        agent: task.agent_type.as_str().to_string(),
                        mode: parsed.mode.clone(),
                        workflow: parsed.workflow.clone(),
                        goal: parsed.goal.clone(),
                        prompt_preview: prompt_preview_text.clone(),
                        result_ok: false,
                        error: Some(e.to_string()),
                        created_at: created_at.clone(),
                    },
                );
                return json_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({
                        "error": e.to_string(),
                        "task_id": task_id.to_string(),
                    }),
                );
            }
        }
    } else {
        match goal_spec {
            Some(spec) => match state.runtime.execute_goal_task(task, spec).await {
                Ok((r, progress)) => {
                    if let Ok(mut value) = serde_json::to_value(&r) {
                        if let Some(obj) = value.as_object_mut() {
                            obj.insert(
                                "goal_progress".to_string(),
                                serde_json::to_value(progress).unwrap_or(json!({})),
                            );
                        }
                    }
                    r
                }
                Err(e) => {
                    push_history(
                        &state,
                        TaskHistoryEntry {
                            task_id: task_id.to_string(),
                            session_id: session_id.to_string(),
                            working_directory: working_dir.to_string_lossy().to_string(),
                            agent: agent_for_history.clone(),
                            mode: parsed.mode.clone(),
                            workflow: parsed.workflow.clone(),
                            goal: parsed.goal.clone(),
                            prompt_preview: prompt_preview_text.clone(),
                            result_ok: false,
                            error: Some(e.to_string()),
                            created_at: created_at.clone(),
                        },
                    );
                    return json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({
                            "error": e.to_string(),
                            "task_id": task_id.to_string(),
                        }),
                    );
                }
            },
            None => match state.runtime.execute_task(task).await {
                Ok(r) => r,
                Err(e) => {
                    push_history(
                        &state,
                        TaskHistoryEntry {
                            task_id: task_id.to_string(),
                            session_id: session_id.to_string(),
                            working_directory: working_dir.to_string_lossy().to_string(),
                            agent: agent_for_history.clone(),
                            mode: parsed.mode.clone(),
                            workflow: parsed.workflow.clone(),
                            goal: parsed.goal.clone(),
                            prompt_preview: prompt_preview_text.clone(),
                            result_ok: false,
                            error: Some(e.to_string()),
                            created_at: created_at.clone(),
                        },
                    );
                    return json_response(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({
                            "error": e.to_string(),
                            "task_id": task_id.to_string(),
                        }),
                    );
                }
            },
        }
    };

    let result_ok = matches!(result, TaskResult::Success { .. } | TaskResult::Partial { .. });
    let err_text = match &result {
        TaskResult::Failure { error, .. } => Some(error.clone()),
        _ => None,
    };

    push_history(
        &state,
        TaskHistoryEntry {
            task_id: task_id.to_string(),
            session_id: session_id.to_string(),
            working_directory: working_dir.to_string_lossy().to_string(),
            agent: agent_for_history,
            mode: parsed.mode.clone(),
            workflow: parsed.workflow.clone(),
            goal: parsed.goal.clone(),
            prompt_preview: prompt_preview_text,
            result_ok,
            error: err_text,
            created_at,
        },
    );

    let payload = PostTaskResponse {
        task_id: task_id.to_string(),
        result,
        output_log_path: log_path.to_string_lossy().to_string(),
    };

    match serde_json::to_vec(&payload) {
        Ok(bytes) => Response::builder()
            .status(StatusCode::OK)
            .header("content-type", "application/json; charset=utf-8")
            .body(Body::from(bytes))
            .unwrap(),
        Err(e) => json_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({ "error": format!("serialize: {}", e) }),
        ),
    }
}

fn json_response(status: StatusCode, v: serde_json::Value) -> Response<Body> {
    let bytes = serde_json::to_vec(&v).unwrap_or_else(|_| b"{\"error\":\"serialize\"}".to_vec());
    Response::builder()
        .status(status)
        .header("content-type", "application/json; charset=utf-8")
        .body(Body::from(bytes))
        .unwrap()
}

pub async fn serve(
    addr: SocketAddr,
    runtime: Arc<AgentRuntime>,
    config: Arc<Config>,
) -> anyhow::Result<()> {
    let state = Arc::new(DaemonState {
        runtime,
        config,
        history: Mutex::new(VecDeque::new()),
    });
    let st = state;
    let make_svc = make_service_fn(move |_conn| {
        let inner = st.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let inner = inner.clone();
                async move { handle(req, inner).await }
            }))
        }
    });

    Server::bind(&addr)
        .serve(make_svc)
        .await
        .map_err(|e| anyhow::anyhow!(e))
}
