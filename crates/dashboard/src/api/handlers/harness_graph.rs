//! Product GraphRunner routes. Start and Resume are different APIs.
use super::*;
use crate::api::auth::resolve_request_user_from_headers;
use anycode_harness_cloud818::identity::{
    authorize, ContextKind, Identity, LocalAuthorization, ProductAcl,
};
use anycode_harness_cloud818::metering::UsageReceipt;
use anycode_harness_core::{
    budget::{BudgetPool, BudgetSnapshot},
    journal::MemoryJournal,
    run_store::{FileRunStore, NodeLease, RunRecord},
    Capabilities, RunContext,
};
use anycode_harness_extensions::{
    checkpoint::FileCheckpoint,
    graph::{Checkpoint, Graph, GraphRunner, GraphStatus},
};
use axum::http::HeaderMap;
use axum::response::Response;
use sha2::Digest;
use std::path::{Path as FsPath, PathBuf};
use std::time::Duration;
use uuid::Uuid;

const MAX_GRAPH_BYTES: usize = 256 * 1024;

#[derive(Deserialize)]
pub struct HarnessGraphStartRequest {
    pub graph: serde_json::Value,
    pub session_id: Option<String>,
}

#[derive(Deserialize)]
pub struct HarnessGraphResumeRequest {
    pub session_id: Option<String>,
}

#[derive(Deserialize)]
pub struct HarnessGraphResolveRequest {
    pub session_id: Option<String>,
    pub revision: u64,
    pub node_id: String,
    pub approved: bool,
}

#[derive(Deserialize)]
pub struct HarnessComputerTicketRequest {
    pub run_id: Option<String>,
    pub ttl_secs: Option<u64>,
}

pub async fn get_harness_status(State(state): State<AppState>) -> impl IntoResponse {
    Json(json!({
        "graph": state.harness_graph,
        "gray_projects": state.harness_gray_projects,
        "unified_kernel": state.harness_unified_kernel,
        "computer": false,
        "x11_verified": false,
        "macos_screencapture": cfg!(target_os = "macos")
            && std::path::Path::new("/usr/sbin/screencapture").is_file(),
        "accounts_sso": state.accounts_sso.is_some(),
        "pairing": if state.accounts_sso.is_some() {
            "accounts_sso"
        } else {
            "local"
        },
        "run_store": "file",
        "note": "Graph routes are opt-in. Computer backends are not enrolled; macOS observe exists only after host enroll. accounts_sso true means a server-side introspect client is attached, not that live 818cloud tenants/wallets are cut over. FileRunStore usage_fact outbox is not wallet settlement or cluster fencing."
    }))
}

pub async fn start_harness_graph(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(body): Json<HarnessGraphStartRequest>,
) -> impl IntoResponse {
    match start_harness_graph_inner(&state, &headers, &project_id, body).await {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub async fn resume_harness_graph(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, run_id)): Path<(String, String)>,
    Json(body): Json<HarnessGraphResumeRequest>,
) -> impl IntoResponse {
    match resume_harness_graph_inner(&state, &headers, &project_id, &run_id, body).await {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub async fn resolve_harness_graph(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, run_id)): Path<(String, String)>,
    Json(body): Json<HarnessGraphResolveRequest>,
) -> impl IntoResponse {
    match resolve_harness_graph_inner(&state, &headers, &project_id, &run_id, body).await {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

pub async fn get_harness_graph(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((project_id, run_id)): Path<(String, String)>,
) -> impl IntoResponse {
    if let Err(resp) = authorize_project(&state, &headers, &project_id, None).await {
        return resp;
    }
    let dir = run_dir(&state.tasks_root, &project_id, &run_id);
    let def = dir.join("definition.json");
    let cp = dir.join("checkpoint.json");
    if !def.is_file() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "graph run not found" })),
        )
            .into_response();
    }
    let graph: serde_json::Value = match std::fs::read_to_string(&def)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(v) => v,
        None => {
            return (
                StatusCode::CONFLICT,
                Json(json!({ "error": "corrupt graph definition" })),
            )
                .into_response();
        }
    };
    let checkpoint = std::fs::read_to_string(&cp)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok());
    Json(json!({ "graph": graph, "checkpoint": checkpoint })).into_response()
}

async fn start_harness_graph_inner(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    body: HarnessGraphStartRequest,
) -> std::result::Result<serde_json::Value, Response> {
    let (project, ctx) =
        authorize_graph(state, headers, project_id, body.session_id.as_deref(), None).await?;
    let raw = serde_json::to_vec(&body.graph)
        .map_err(|_| bad(StatusCode::BAD_REQUEST, "graph is not JSON"))?;
    if raw.len() > MAX_GRAPH_BYTES {
        return Err(bad(StatusCode::PAYLOAD_TOO_LARGE, "graph exceeds 256KiB"));
    }
    let graph: Graph = serde_json::from_value(body.graph)
        .map_err(|e| bad(StatusCode::BAD_REQUEST, &format!("invalid graph: {e}")))?;
    graph
        .validate()
        .map_err(|e| bad(StatusCode::BAD_REQUEST, &e.to_string()))?;
    let run_id = Uuid::new_v4();
    let dir = run_dir(&state.tasks_root, project_id, &run_id.to_string());
    std::fs::create_dir_all(&dir)
        .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    std::fs::write(
        dir.join("definition.json"),
        serde_json::to_vec(&graph).unwrap_or_default(),
    )
    .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let store = FileCheckpoint::open(&dir.join("checkpoint.json"))
        .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
    let (executor, journal) = product_executor(state, PathBuf::from(&project.root_path));
    let runner = GraphRunner {
        executor: &executor,
        store: &store,
    };
    let result = runner
        .start_with_run(&graph, &ctx, run_id)
        .await
        .map_err(map_graph_err)?;
    persist_run_store(
        &state.tasks_root,
        project_id,
        &result.checkpoint,
        &result.status,
    )?;
    persist_usage_facts(
        &state.tasks_root,
        journal.as_ref(),
        result.checkpoint.run_id,
    )?;
    release_terminal_computer_run(
        state,
        &project.root_path,
        &result.status,
        result.checkpoint.run_id,
    )
    .await;
    Ok(graph_json(&result.status, &result.checkpoint))
}

async fn resume_harness_graph_inner(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    run_id: &str,
    body: HarnessGraphResumeRequest,
) -> std::result::Result<serde_json::Value, Response> {
    let expected = Uuid::parse_str(run_id)
        .map_err(|_| bad(StatusCode::BAD_REQUEST, "run_id must be a UUID"))?;
    let dir = run_dir(&state.tasks_root, project_id, run_id);
    let prior = load_checkpoint(&dir)?;
    let (project, ctx) = authorize_graph(
        state,
        headers,
        project_id,
        body.session_id.as_deref(),
        Some(prior.budget),
    )
    .await?;
    let graph = load_definition(&dir)?;
    let store = FileCheckpoint::open(&dir.join("checkpoint.json"))
        .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
    let _lease = acquire_graph_lease(&state.tasks_root, expected)?;
    let (executor, journal) = product_executor(state, PathBuf::from(&project.root_path));
    let runner = GraphRunner {
        executor: &executor,
        store: &store,
    };
    let result = runner
        .resume(&graph, &ctx, expected)
        .await
        .map_err(map_graph_err)?;
    persist_run_store(
        &state.tasks_root,
        project_id,
        &result.checkpoint,
        &result.status,
    )?;
    persist_usage_facts(
        &state.tasks_root,
        journal.as_ref(),
        result.checkpoint.run_id,
    )?;
    release_terminal_computer_run(
        state,
        &project.root_path,
        &result.status,
        result.checkpoint.run_id,
    )
    .await;
    Ok(graph_json(&result.status, &result.checkpoint))
}

async fn resolve_harness_graph_inner(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    run_id: &str,
    body: HarnessGraphResolveRequest,
) -> std::result::Result<serde_json::Value, Response> {
    let expected = Uuid::parse_str(run_id)
        .map_err(|_| bad(StatusCode::BAD_REQUEST, "run_id must be a UUID"))?;
    let dir = run_dir(&state.tasks_root, project_id, run_id);
    let prior = load_checkpoint(&dir)?;
    let (project, ctx) = authorize_graph(
        state,
        headers,
        project_id,
        body.session_id.as_deref(),
        Some(prior.budget),
    )
    .await?;
    let graph = load_definition(&dir)?;
    let store = FileCheckpoint::open(&dir.join("checkpoint.json"))
        .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
    let _lease = acquire_graph_lease(&state.tasks_root, expected)?;
    let (executor, _) = product_executor(state, PathBuf::from(&project.root_path));
    let runner = GraphRunner {
        executor: &executor,
        store: &store,
    };
    let checkpoint = runner
        .resolve_human(
            &graph,
            &ctx,
            expected,
            body.revision,
            &body.node_id,
            body.approved,
        )
        .map_err(map_graph_err)?;
    persist_run_store(
        &state.tasks_root,
        project_id,
        &checkpoint,
        &GraphStatus::Waiting,
    )?;
    Ok(json!({
        "status": "waiting_or_ready",
        "checkpoint": checkpoint,
        "approver": ctx.scope().subject,
    }))
}

pub async fn issue_harness_computer_ticket(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(project_id): Path<String>,
    Json(body): Json<HarnessComputerTicketRequest>,
) -> impl IntoResponse {
    match issue_harness_computer_ticket_inner(&state, &headers, &project_id, body).await {
        Ok(value) => (StatusCode::OK, Json(value)).into_response(),
        Err(resp) => resp,
    }
}

async fn release_terminal_computer_run(
    state: &AppState,
    project_root: &str,
    status: &GraphStatus,
    run: Uuid,
) {
    if !matches!(
        status,
        GraphStatus::Completed
            | GraphStatus::Failed
            | GraphStatus::Cancelled
            | GraphStatus::Partial
            | GraphStatus::Uncertain
    ) {
        return;
    }
    if let Some(runtime) = state
        .chat_runtime
        .existing_runtime(std::path::Path::new(project_root))
        .await
    {
        runtime.unbind_harness_product_run(run);
    }
}

fn computer_not_enrolled() -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(json!({
            "error": "computer backends are not enrolled; tickets are not issued",
            "computer": false,
            "x11_verified": false,
        })),
    )
        .into_response()
}

async fn issue_harness_computer_ticket_inner(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    body: HarnessComputerTicketRequest,
) -> std::result::Result<serde_json::Value, Response> {
    let (project, authorized) = authorize_project(state, headers, project_id, None).await?;
    let Some(runtime) = state
        .chat_runtime
        .existing_runtime(std::path::Path::new(&project.root_path))
        .await
    else {
        return Err(computer_not_enrolled());
    };
    let Some(kind) = runtime.computer_backend_kind() else {
        return Err(computer_not_enrolled());
    };
    let Some(device) = authorized.scope.device else {
        return Err(bad(
            StatusCode::FORBIDDEN,
            "computer tickets require a paired device in scope",
        ));
    };
    let run_id = body
        .run_id
        .as_deref()
        .ok_or_else(|| bad(StatusCode::BAD_REQUEST, "run_id required"))?;
    let run = Uuid::parse_str(run_id)
        .map_err(|_| bad(StatusCode::BAD_REQUEST, "run_id must be a UUID"))?;
    let store = open_run_store(&state.tasks_root)?;
    let record = store
        .load(run)
        .map_err(|_| bad(StatusCode::NOT_FOUND, "harness run not found"))?;
    if record
        .payload
        .get("project_id")
        .and_then(|v| v.as_str())
        .is_some_and(|id| id != project.id)
    {
        return Err(bad(
            StatusCode::FORBIDDEN,
            "run does not belong to this project",
        ));
    }
    let status = record
        .payload
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if matches!(
        status,
        "completed" | "failed" | "cancelled" | "partial" | "uncertain"
    ) {
        return Err(bad(StatusCode::CONFLICT, "run is not in-flight"));
    }
    let ttl = body.ttl_secs.unwrap_or(60);
    let ctx = RunContext::root(
        authorized.scope,
        authorized.capabilities,
        BudgetPool::new(1_000_000)
            .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?,
        Duration::from_secs(60),
    )
    .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let issued = runtime
        .issue_harness_computer_ticket_for_run(
            &ctx,
            std::path::Path::new(&project.root_path),
            device,
            ttl,
            run,
        )
        .map_err(|e| match e {
            anycode_harness_core::Error::Denied(msg) => bad(StatusCode::FORBIDDEN, &msg),
            anycode_harness_core::Error::Invalid(msg) => bad(StatusCode::BAD_REQUEST, &msg),
            other => bad(StatusCode::CONFLICT, &other.to_string()),
        })?;
    if issued.run != run || issued.device != device {
        return Err(bad(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ticket was not bound to the requested run and device",
        ));
    }
    Ok(json!({
        "ticket": issued.ticket,
        "run_id": issued.run,
        "device": issued.device,
        "ttl_secs": ttl,
        "backend": kind,
        "x11_verified": false,
        "note": "ticket is host-issued and bound to this in-flight run; X11 is not verified"
    }))
}

/// Local Workbench membership: the authenticated user plus an existing SQLite
/// project. When an 818cloud SSO client is attached, personal introspect is
/// still not enough — `harness_accounts_members` must contain the subject.
/// Enterprise tenant grants stay fail-closed until live grant versions exist.
/// This is not wallet settlement.
struct LocalWorkbenchAcl {
    project: Uuid,
    project_id: String,
    db: Option<crate::db::DashboardDb>,
    require_accounts_membership: bool,
}

#[async_trait::async_trait]
impl ProductAcl for LocalWorkbenchAcl {
    async fn authorize_project(
        &self,
        identity: &Identity,
        project: Uuid,
    ) -> anycode_harness_core::Result<LocalAuthorization> {
        if project != self.project {
            return Err(anycode_harness_core::Error::Denied(
                "ACL returned wrong project".into(),
            ));
        }
        if identity.context != ContextKind::Personal || identity.tenant.is_some() {
            return Err(anycode_harness_core::Error::Denied(
                "enterprise requires live ProductAcl grant versions".into(),
            ));
        }
        if self.require_accounts_membership {
            let Some(db) = &self.db else {
                return Err(anycode_harness_core::Error::Denied(
                    "SSO membership store is not attached".into(),
                ));
            };
            let member = db
                .harness_accounts_member(&self.project_id, &identity.sub.to_string())
                .await
                .map_err(|e| anycode_harness_core::Error::Host(e.to_string()))?;
            if !member {
                return Err(anycode_harness_core::Error::Denied(
                    "not a project member".into(),
                ));
            }
        }
        Ok(LocalAuthorization {
            project,
            capabilities: Capabilities::new([
                "agent.spawn".into(),
                "fs.read".into(),
                "skill.read".into(),
            ])?,
        })
    }
}

async fn authorize_project(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    session_id: Option<&str>,
) -> std::result::Result<
    (
        crate::schema::ProjectDetail,
        anycode_harness_cloud818::identity::AuthorizedScope,
    ),
    Response,
> {
    if !state.harness_graph {
        return Err((
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "harness graph routes are not armed" })),
        )
            .into_response());
    }
    if !state.harness_gray_projects.is_empty()
        && !anycode_core::harness_gray_project_enabled(project_id, &state.harness_gray_projects)
    {
        return Err(bad(
            StatusCode::FORBIDDEN,
            "project is not in the harness gray allowlist",
        ));
    }
    let project = state
        .db
        .get_project(project_id)
        .await
        .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
        .ok_or_else(|| bad(StatusCode::NOT_FOUND, "project not found"))?;
    if let Some(session_id) = session_id {
        let session = state
            .db
            .get_session(session_id)
            .await
            .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?
            .ok_or_else(|| bad(StatusCode::NOT_FOUND, "session not found"))?;
        if session.project_id != project.id {
            return Err(bad(
                StatusCode::FORBIDDEN,
                "session does not belong to this project",
            ));
        }
    }
    let project_uuid = stable_uuid(&project.id);
    let acl = LocalWorkbenchAcl {
        project: project_uuid,
        project_id: project.id.clone(),
        db: Some(state.db.clone()),
        require_accounts_membership: state.accounts_sso.is_some(),
    };
    if let Some(client) = &state.accounts_sso {
        let token = bearer_opaque_token(headers).ok_or_else(|| {
            bad(
                StatusCode::UNAUTHORIZED,
                "818cloud opaque bearer token required",
            )
        })?;
        let identity = client
            .introspect(token, None)
            .await
            .map_err(|e| bad(StatusCode::FORBIDDEN, &e.to_string()))?;
        let device =
            super::resolve_harness_device(state, headers, &identity.sub.to_string()).await?;
        let authorized = authorize(&identity, client.issuer(), None, project_uuid, device, &acl)
            .await
            .map_err(|e| bad(StatusCode::FORBIDDEN, &e.to_string()))?;
        return Ok((project, authorized));
    }
    let user = resolve_request_user_from_headers(state, headers)
        .await
        .ok_or_else(|| bad(StatusCode::UNAUTHORIZED, "authenticated user required"))?;
    let identity = Identity {
        active: true,
        iss: "anycode.local".into(),
        sub: stable_uuid(&user.id),
        aud: "anycode".into(),
        context: ContextKind::Personal,
        tenant: None,
        scope: "identity".into(),
    };
    let device = super::resolve_harness_device(state, headers, &identity.sub.to_string()).await?;
    let authorized = authorize(&identity, "anycode.local", None, project_uuid, device, &acl)
        .await
        .map_err(|e| bad(StatusCode::FORBIDDEN, &e.to_string()))?;
    Ok((project, authorized))
}

pub(crate) fn bearer_opaque_token(headers: &HeaderMap) -> Option<&str> {
    let raw = headers
        .get(axum::http::header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .trim();
    raw.strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .map(str::trim)
        .filter(|t| !t.is_empty())
}

async fn authorize_graph(
    state: &AppState,
    headers: &HeaderMap,
    project_id: &str,
    session_id: Option<&str>,
    budget: Option<BudgetSnapshot>,
) -> std::result::Result<(crate::schema::ProjectDetail, RunContext), Response> {
    let (project, authorized) = authorize_project(state, headers, project_id, session_id).await?;
    let pool = match budget {
        Some(snapshot) => BudgetPool::from_checkpoint(snapshot)
            .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?,
        None => BudgetPool::new(1_000_000)
            .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?,
    };
    let ctx = RunContext::root(
        authorized.scope,
        authorized.capabilities,
        pool,
        Duration::from_secs(3600),
    )
    .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    Ok((project, ctx))
}

fn load_checkpoint(dir: &FsPath) -> std::result::Result<Checkpoint, Response> {
    let bytes = std::fs::read(dir.join("checkpoint.json"))
        .map_err(|_| bad(StatusCode::NOT_FOUND, "graph checkpoint missing"))?;
    serde_json::from_slice(&bytes).map_err(|_| bad(StatusCode::CONFLICT, "corrupt checkpoint"))
}

fn persist_run_store(
    tasks_root: &FsPath,
    project_id: &str,
    checkpoint: &Checkpoint,
    status: &GraphStatus,
) -> std::result::Result<(), Response> {
    let root = tasks_root.join("harness").join("runs");
    std::fs::create_dir_all(&root)
        .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let abs = std::fs::canonicalize(&root)
        .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let store = FileRunStore::open(&abs).map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
    let status_slug = format!("{status:?}").to_ascii_lowercase();
    let payload = serde_json::json!({
        "project_id": project_id,
        "status": status_slug,
        "checkpoint_revision": checkpoint.revision,
    });
    let store_revision = match store.load(checkpoint.run_id) {
        Ok(current) => {
            let expected = current.revision;
            let mut next = current;
            next.revision = expected + 1;
            next.budget = checkpoint.budget;
            next.payload = payload.clone();
            store
                .cas(expected, &next)
                .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
            expected + 1
        }
        Err(_) => {
            store
                .create(&RunRecord {
                    run_id: checkpoint.run_id,
                    revision: 0,
                    fence: 0,
                    node_lease: None,
                    budget: checkpoint.budget,
                    payload: payload.clone(),
                })
                .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
            0
        }
    };
    store
        .append_outbox(
            checkpoint.run_id,
            "graph_status",
            payload,
            &format!("{}:{}:{}", checkpoint.run_id, store_revision, status_slug),
        )
        .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
    Ok(())
}

struct GraphLease {
    store: FileRunStore,
    run_id: Uuid,
    token: Uuid,
}

impl Drop for GraphLease {
    fn drop(&mut self) {
        let _ = self.store.release_node_lease(self.run_id, self.token);
    }
}

fn open_run_store(tasks_root: &FsPath) -> std::result::Result<FileRunStore, Response> {
    let root = tasks_root.join("harness").join("runs");
    std::fs::create_dir_all(&root)
        .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    let abs = std::fs::canonicalize(&root)
        .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
    FileRunStore::open(&abs).map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))
}

fn acquire_graph_lease(
    tasks_root: &FsPath,
    run_id: Uuid,
) -> std::result::Result<GraphLease, Response> {
    let store = open_run_store(tasks_root)?;
    let lease: NodeLease = store
        .acquire_node_lease(run_id, "graph", 30)
        .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
    Ok(GraphLease {
        store,
        run_id,
        token: lease.token,
    })
}

fn load_definition(dir: &FsPath) -> std::result::Result<Graph, Response> {
    let bytes = std::fs::read(dir.join("definition.json"))
        .map_err(|_| bad(StatusCode::NOT_FOUND, "graph definition missing"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| bad(StatusCode::CONFLICT, "corrupt graph definition"))
}

fn run_dir(tasks_root: &FsPath, project_id: &str, run_id: &str) -> PathBuf {
    tasks_root
        .join("harness")
        .join("graphs")
        .join(project_id)
        .join(run_id)
}

pub(crate) fn stable_uuid(name: &str) -> Uuid {
    if let Ok(id) = Uuid::parse_str(name) {
        return id;
    }
    let digest = sha2::Sha256::digest(name.as_bytes());
    let mut bytes = [0u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    Uuid::from_bytes(bytes)
}

fn graph_json(
    status: &GraphStatus,
    checkpoint: &anycode_harness_extensions::graph::Checkpoint,
) -> serde_json::Value {
    json!({
        "status": format!("{status:?}").to_ascii_lowercase(),
        "run_id": checkpoint.run_id,
        "revision": checkpoint.revision,
        "checkpoint": checkpoint,
    })
}

fn map_graph_err(err: anycode_harness_core::Error) -> Response {
    let code = match &err {
        anycode_harness_core::Error::Denied(_) => StatusCode::FORBIDDEN,
        anycode_harness_core::Error::Conflict(_) => StatusCode::CONFLICT,
        anycode_harness_core::Error::Invalid(_) => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    bad(code, &err.to_string())
}

fn bad(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "error": message }))).into_response()
}

fn product_executor(
    state: &AppState,
    root: PathBuf,
) -> (ProductNodeExecutor, std::sync::Arc<MemoryJournal>) {
    let journal = std::sync::Arc::new(MemoryJournal::default());
    (
        ProductNodeExecutor {
            root,
            chat: state.chat_runtime.clone(),
            journal: journal.clone(),
        },
        journal,
    )
}

fn persist_usage_facts(
    tasks_root: &FsPath,
    journal: &MemoryJournal,
    run_id: Uuid,
) -> std::result::Result<(), Response> {
    let store = open_run_store(tasks_root)?;
    let records = journal
        .records()
        .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
    for record in records {
        let Ok(receipt) =
            UsageReceipt::from_measured_event(&record.event, "harness-kernel", "measured")
        else {
            continue;
        };
        if !receipt.billable() {
            continue;
        }
        let payload = serde_json::to_value(&receipt)
            .map_err(|e| bad(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
        if payload.get("cny").is_some() || payload.get("wallet").is_some() {
            return Err(bad(
                StatusCode::INTERNAL_SERVER_ERROR,
                "usage fact must not carry wallet fields",
            ));
        }
        store
            .append_outbox(run_id, "usage_fact", payload, &receipt.idempotency_key)
            .map_err(|e| bad(StatusCode::CONFLICT, &e.to_string()))?;
    }
    Ok(())
}

struct ProductNodeExecutor {
    root: PathBuf,
    chat: crate::control::chat_runtime::ChatRuntimeHost,
    journal: std::sync::Arc<MemoryJournal>,
}

#[async_trait::async_trait]
impl anycode_harness_extensions::graph::NodeExecutor for ProductNodeExecutor {
    async fn execute(
        &self,
        ctx: &RunContext,
        node: &anycode_harness_extensions::graph::Node,
        inputs: std::collections::BTreeMap<String, serde_json::Value>,
    ) -> anycode_harness_core::Result<anycode_harness_extensions::graph::NodeOutput> {
        let runtime = self.chat.runtime_for_gate(&self.root).await.map_err(|e| {
            anycode_harness_core::Error::Denied(format!("work nodes require a Kernel host: {e}"))
        })?;
        let factory = anycode_agent::RuntimeHostFactory::new(
            runtime,
            self.root.clone(),
            anycode_harness_core::events::PreviewBus::default(),
        );
        let exec = anycode_harness_host::graph_adapter::KernelNodeExecutor {
            factory: std::sync::Arc::new(factory),
            journal: self.journal.clone(),
            previews: anycode_harness_core::events::PreviewBus::default(),
            limits: anycode_harness_core::types::Limits {
                max_turns: 16,
                ..anycode_harness_core::types::Limits::default()
            },
        };
        exec.execute(ctx, node, inputs).await
    }

    async fn verify(
        &self,
        ctx: &RunContext,
        verifier: &str,
        inputs: std::collections::BTreeMap<String, serde_json::Value>,
    ) -> anycode_harness_core::Result<anycode_harness_extensions::graph::Verification> {
        ctx.check()?;
        anycode_agent::verify_trusted_artifact(&self.root, verifier, &inputs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CreateSessionRequest, UpsertProjectRequest};
    use crate::server::{app_for_test_custom, TestAppOptions};
    use axum::body::Body;
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    fn human_graph() -> serde_json::Value {
        json!({
            "version": 1,
            "name": "human-pause",
            "nodes": [{
                "id": "ask",
                "kind": {"type": "human", "question": "Ship this change?"}
            }]
        })
    }

    fn work_graph() -> serde_json::Value {
        json!({
            "version": 1,
            "name": "untrusted-worker",
            "nodes": [{
                "id": "do",
                "kind": {"type": "work", "agent": "worker", "prompt": "do not invent a second loop"}
            }]
        })
    }

    fn gate_graph() -> serde_json::Value {
        json!({
            "version": 1,
            "name": "sha-gate",
            "nodes": [{
                "id": "check",
                "kind": {"type": "gate", "verifier": "artifact.sha256"}
            }]
        })
    }

    fn branch_any_graph() -> serde_json::Value {
        json!({
            "version": 1,
            "name": "product-branch-any",
            "max_parallel": 2,
            "nodes": [
                {"id":"classify","kind":{"type":"work","agent":"explore","prompt":"classify"}},
                {"id":"branch","kind":{"type":"branch","source":"classify","pointer":"/text","equals":"needs-change"},"depends_on":[{"node":"classify"}]},
                {"id":"repair","kind":{"type":"work","agent":"explore","prompt":"repair"},"depends_on":[{"node":"branch","on":"true"}]},
                {"id":"audit","kind":{"type":"work","agent":"explore","prompt":"audit"},"depends_on":[{"node":"branch","on":"false"}]},
                {"id":"summary","kind":{"type":"work","agent":"explore","prompt":"summary"},"depends_on":[{"node":"repair"},{"node":"audit"}],"join":"any"}
            ]
        })
    }

    fn all_join_graph() -> serde_json::Value {
        json!({
            "version": 1,
            "name": "product-all-join",
            "max_parallel": 2,
            "nodes": [
                {"id":"left","kind":{"type":"work","agent":"explore","prompt":"left"}},
                {"id":"right","kind":{"type":"work","agent":"explore","prompt":"right"}},
                {"id":"join","kind":{"type":"work","agent":"explore","prompt":"join"},"depends_on":[{"node":"left"},{"node":"right"}],"join":"all"}
            ]
        })
    }

    fn two_work_graph() -> serde_json::Value {
        json!({
            "version": 1,
            "name": "product-partial-uncertain",
            "nodes": [
                {"id":"first","kind":{"type":"work","agent":"explore","prompt":"first"}},
                {"id":"second","kind":{"type":"work","agent":"explore","prompt":"second"},"depends_on":[{"node":"first"}]}
            ]
        })
    }

    struct ScriptedLlm {
        texts: std::sync::Mutex<Vec<String>>,
    }

    impl ScriptedLlm {
        fn new(texts: &[&str]) -> Self {
            Self {
                texts: std::sync::Mutex::new(texts.iter().map(|s| (*s).to_string()).collect()),
            }
        }

        fn pop(&self) -> Result<String, anycode_core::CoreError> {
            self.texts
                .lock()
                .ok()
                .and_then(|mut q| {
                    if q.is_empty() {
                        None
                    } else {
                        Some(q.remove(0))
                    }
                })
                .ok_or_else(|| anycode_core::CoreError::LLMError("scripted queue empty".into()))
        }
    }

    #[async_trait::async_trait]
    impl anycode_core::LLMClient for ScriptedLlm {
        async fn chat(
            &self,
            _messages: Vec<anycode_core::Message>,
            _tools: Vec<anycode_core::ToolSchema>,
            _config: &anycode_core::ModelConfig,
        ) -> Result<anycode_core::LLMResponse, anycode_core::CoreError> {
            let text = self.pop()?;
            Ok(anycode_core::LLMResponse {
                message: anycode_core::Message {
                    id: Uuid::new_v4(),
                    role: anycode_core::MessageRole::Assistant,
                    content: anycode_core::MessageContent::Text(text),
                    timestamp: chrono::Utc::now(),
                    metadata: Default::default(),
                },
                tool_calls: vec![],
                usage: anycode_core::Usage {
                    input_tokens: 2,
                    output_tokens: 1,
                    cache_creation_tokens: None,
                    cache_read_tokens: None,
                },
            })
        }

        async fn chat_stream(
            &self,
            _messages: Vec<anycode_core::Message>,
            _tools: Vec<anycode_core::ToolSchema>,
            _config: &anycode_core::ModelConfig,
        ) -> Result<tokio::sync::mpsc::Receiver<anycode_core::StreamEvent>, anycode_core::CoreError>
        {
            let text = self.pop()?;
            let (tx, rx) = tokio::sync::mpsc::channel(8);
            tokio::spawn(async move {
                let _ = tx.send(anycode_core::StreamEvent::Delta(text)).await;
                let _ = tx
                    .send(anycode_core::StreamEvent::Usage(anycode_core::Usage {
                        input_tokens: 2,
                        output_tokens: 1,
                        cache_creation_tokens: None,
                        cache_read_tokens: None,
                    }))
                    .await;
                let _ = tx.send(anycode_core::StreamEvent::Done).await;
            });
            Ok(rx)
        }
    }

    async fn json_req(
        app: axum::Router,
        method: &str,
        uri: &str,
        body: serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let res = app
            .oneshot(
                axum::http::Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let json = serde_json::from_slice(&bytes).unwrap_or(json!({}));
        (status, json)
    }

    async fn seed_project(
        db: &crate::db::DashboardDb,
        root: &FsPath,
        name: &str,
    ) -> (crate::schema::ProjectDetail, crate::schema::SessionDetail) {
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: root.to_string_lossy().into(),
                name: Some(name.into()),
                create_root: Some(true),
                ..Default::default()
            })
            .await
            .unwrap();
        let session = db
            .create_session(CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "repl".into(),
                task_id: None,
                title: name.into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await
            .unwrap();
        (project, session)
    }

    #[tokio::test]
    async fn graph_routes_are_hidden_until_armed() {
        let dir = tempfile::tempdir().unwrap();
        let app = app_for_test_custom(&dir.path().join("db.sqlite"), TestAppOptions::default())
            .await
            .unwrap();
        let res = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/harness/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["graph"], false);
        assert_eq!(json["unified_kernel"], false);
        assert_eq!(json["computer"], false);
        assert_eq!(json["x11_verified"], false);
        assert!(json["macos_screencapture"].is_boolean());
        assert_eq!(json["accounts_sso"], false);
        assert_eq!(json["pairing"], "local");
        assert_eq!(json["run_store"], "file");
    }

    #[tokio::test]
    async fn harness_status_reports_opt_in_unified_kernel_without_enrolling_computer() {
        let dir = tempfile::tempdir().unwrap();
        let mut opts = TestAppOptions::default();
        opts.harness_unified_kernel = true;
        let app = app_for_test_custom(&dir.path().join("db.sqlite"), opts)
            .await
            .unwrap();
        let res = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/harness/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["unified_kernel"], true);
        assert_eq!(json["graph"], false);
        assert_eq!(json["computer"], false);
    }

    #[tokio::test]
    async fn human_start_resume_resolve_and_isolation() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks.clone());
        let app = app_for_test_custom(&db_path, opts).await.unwrap();
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let (project, session) = seed_project(&db, &tmp.path().join("proj"), "Graph").await;
        let (other, _) = seed_project(&db, &tmp.path().join("other"), "Other").await;

        let (status, started) = json_req(
            app.clone(),
            "POST",
            &format!("/api/projects/{}/harness/graphs/start", project.id),
            json!({"graph": human_graph(), "session_id": session.id}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{started}");
        assert!(
            started["status"].as_str().unwrap_or("").contains("waiting"),
            "{started}"
        );
        let run_id = started["run_id"].as_str().expect("run_id");
        assert!(
            tasks
                .join("harness/runs")
                .join(run_id)
                .join("record.json")
                .is_file(),
            "FileRunStore must persist the graph run"
        );
        let (evil_status, evil) = {
            let res = app
                .clone()
                .oneshot(
                    axum::http::Request::builder()
                        .method("POST")
                        .uri(format!("/api/projects/{}/harness/graphs/start", project.id))
                        .header("content-type", "application/json")
                        .header("origin", "https://evil.example")
                        .body(Body::from(
                            serde_json::to_vec(&json!({
                                "graph": human_graph(),
                                "session_id": session.id
                            }))
                            .unwrap(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            let status = res.status();
            let bytes = res.into_body().collect().await.unwrap().to_bytes();
            (
                status,
                serde_json::from_slice::<serde_json::Value>(&bytes).unwrap_or(json!({})),
            )
        };
        assert_eq!(evil_status, StatusCode::FORBIDDEN, "{evil}");
        let (ticket_status, ticket) = json_req(
            app.clone(),
            "POST",
            &format!("/api/projects/{}/harness/computer/tickets", project.id),
            json!({}),
        )
        .await;
        assert_eq!(ticket_status, StatusCode::NOT_FOUND, "{ticket}");
        assert_eq!(ticket["computer"], false);
        assert_eq!(ticket["x11_verified"], false);
        let (get_status, fetched) = json_req(
            app.clone(),
            "GET",
            &format!("/api/projects/{}/harness/graphs/{run_id}", project.id),
            json!({}),
        )
        .await;
        assert_eq!(get_status, StatusCode::OK, "{fetched}");
        let revision = started["revision"].as_u64().expect("revision");

        let (resume_status, resumed) = json_req(
            app.clone(),
            "POST",
            &format!(
                "/api/projects/{}/harness/graphs/{run_id}/resume",
                project.id
            ),
            json!({"session_id": session.id}),
        )
        .await;
        assert_eq!(resume_status, StatusCode::OK, "{resumed}");
        assert!(
            resumed["status"].as_str().unwrap_or("").contains("waiting"),
            "resume is not start: {resumed}"
        );
        assert_eq!(resumed["run_id"], started["run_id"]);

        let (cross_status, _) = json_req(
            app.clone(),
            "POST",
            &format!("/api/projects/{}/harness/graphs/{run_id}/resume", other.id),
            json!({}),
        )
        .await;
        assert!(
            matches!(
                cross_status,
                StatusCode::NOT_FOUND | StatusCode::FORBIDDEN | StatusCode::CONFLICT
            ),
            "{cross_status}"
        );

        let (resolve_status, resolved) = json_req(
            app.clone(),
            "POST",
            &format!(
                "/api/projects/{}/harness/graphs/{run_id}/resolve",
                project.id
            ),
            json!({
                "session_id": session.id,
                "revision": revision,
                "node_id": "ask",
                "approved": true
            }),
        )
        .await;
        assert_eq!(resolve_status, StatusCode::OK, "{resolved}");
        assert_eq!(
            resolved["checkpoint"]["nodes"]["ask"]["output"]["approved"],
            true
        );
        assert!(resolved["approver"].as_str().is_some());

        let (after_status, after) = json_req(
            app.clone(),
            "POST",
            &format!(
                "/api/projects/{}/harness/graphs/{run_id}/resume",
                project.id
            ),
            json!({"session_id": session.id}),
        )
        .await;
        assert_eq!(after_status, StatusCode::OK, "{after}");
        assert!(
            after["status"].as_str().unwrap_or("").contains("completed"),
            "{after}"
        );

        std::fs::write(
            tasks
                .join("harness/graphs")
                .join(&project.id)
                .join(run_id)
                .join("definition.json"),
            serde_json::to_vec(&human_graph()).unwrap(),
        )
        .unwrap();
        // Same JSON still matches fingerprint. Change the question to break the digest.
        let mut changed = human_graph();
        changed["nodes"][0]["kind"]["question"] = json!("Tampered?");
        std::fs::write(
            tasks
                .join("harness/graphs")
                .join(&project.id)
                .join(run_id)
                .join("definition.json"),
            serde_json::to_vec(&changed).unwrap(),
        )
        .unwrap();
        let (changed_status, _) = json_req(
            app.clone(),
            "POST",
            &format!(
                "/api/projects/{}/harness/graphs/{run_id}/resume",
                project.id
            ),
            json!({"session_id": session.id}),
        )
        .await;
        assert_eq!(changed_status, StatusCode::CONFLICT);

        std::fs::write(
            tasks
                .join("harness/graphs")
                .join(&project.id)
                .join(run_id)
                .join("checkpoint.json"),
            "{",
        )
        .unwrap();
        let (bad_status, _) = json_req(
            app,
            "POST",
            &format!(
                "/api/projects/{}/harness/graphs/{run_id}/resume",
                project.id
            ),
            json!({"session_id": session.id}),
        )
        .await;
        assert!(
            matches!(
                bad_status,
                StatusCode::CONFLICT | StatusCode::BAD_REQUEST | StatusCode::INTERNAL_SERVER_ERROR
            ),
            "{bad_status}"
        );
    }

    #[tokio::test]
    async fn gate_hashes_real_evidence_and_untrusted_worker_fails_closed() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks = tmp.path().join("tasks");
        let proj = tmp.path().join("proj");
        std::fs::create_dir_all(proj.join(".anycode/harness")).unwrap();
        std::fs::write(proj.join(".anycode/harness/evidence"), b"product-gate").unwrap();
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks);
        let app = app_for_test_custom(&db_path, opts).await.unwrap();
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let (project, session) = seed_project(&db, &proj, "Gate").await;

        let (gate_status, gated) = json_req(
            app.clone(),
            "POST",
            &format!("/api/projects/{}/harness/graphs/start", project.id),
            json!({"graph": gate_graph(), "session_id": session.id}),
        )
        .await;
        assert_eq!(gate_status, StatusCode::OK, "{gated}");
        assert!(
            gated["status"].as_str().unwrap_or("").contains("completed"),
            "{gated}"
        );
        let digest = gated["checkpoint"]["nodes"]["check"]["output"]["artifact_digest"]
            .as_str()
            .unwrap_or("");
        assert_eq!(digest.len(), 64, "{gated}");

        let (work_status, work) = json_req(
            app,
            "POST",
            &format!("/api/projects/{}/harness/graphs/start", project.id),
            json!({"graph": work_graph(), "session_id": session.id}),
        )
        .await;
        assert_eq!(work_status, StatusCode::OK, "{work}");
        assert!(
            work["status"].as_str().unwrap_or("").contains("failed"),
            "untrusted worker must fail closed, got {work}"
        );
    }

    #[tokio::test]
    async fn product_route_branch_any_and_all_join_use_kernel_work_nodes() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let llm = std::sync::Arc::new(ScriptedLlm::new(&[
            "needs-change",
            "repaired",
            "summarized",
            "left-ok",
            "right-ok",
            "joined",
        ]));
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks.clone());
        opts.seeded_runtime = Some(anycode_agent::AgentRuntime::sandboxed_scripted(
            llm,
            std::collections::HashMap::new(),
        ));
        let app = app_for_test_custom(&db_path, opts).await.unwrap();
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let proj = tmp.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        std::fs::write(proj.join("readme.txt"), "product-route kernel work").unwrap();
        let (project, session) = seed_project(&db, &proj, "Branch").await;

        let (branch_status, branched) = json_req(
            app.clone(),
            "POST",
            &format!("/api/projects/{}/harness/graphs/start", project.id),
            json!({"graph": branch_any_graph(), "session_id": session.id}),
        )
        .await;
        assert_eq!(branch_status, StatusCode::OK, "{branched}");
        assert!(
            branched["status"]
                .as_str()
                .unwrap_or("")
                .contains("completed"),
            "{branched}"
        );
        assert_eq!(
            branched["checkpoint"]["nodes"]["classify"]["output"]["text"],
            "needs-change"
        );
        assert_eq!(
            branched["checkpoint"]["nodes"]["repair"]["output"]["text"],
            "repaired"
        );
        assert_eq!(
            branched["checkpoint"]["nodes"]["audit"]["status"],
            "skipped"
        );
        assert_eq!(
            branched["checkpoint"]["nodes"]["summary"]["output"]["text"],
            "summarized"
        );
        let run_id = branched["run_id"].as_str().expect("run_id");
        let outbox: serde_json::Value = serde_json::from_slice(
            &std::fs::read(tasks.join("harness/runs").join(run_id).join("outbox.json")).unwrap(),
        )
        .unwrap();
        let facts: Vec<_> = outbox
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["kind"] == "usage_fact")
            .collect();
        assert_eq!(facts.len(), 3, "{outbox}");
        for fact in facts {
            assert_eq!(fact["payload"]["measured"], true);
            assert!(fact["payload"].get("cny").is_none());
            assert!(fact["payload"].get("wallet").is_none());
            assert!(fact["payload"]["input_tokens"].as_u64().unwrap_or(0) >= 2);
        }

        let (all_status, joined) = json_req(
            app,
            "POST",
            &format!("/api/projects/{}/harness/graphs/start", project.id),
            json!({"graph": all_join_graph(), "session_id": session.id}),
        )
        .await;
        assert_eq!(all_status, StatusCode::OK, "{joined}");
        assert!(
            joined["status"]
                .as_str()
                .unwrap_or("")
                .contains("completed"),
            "{joined}"
        );
        assert_eq!(
            joined["checkpoint"]["nodes"]["left"]["output"]["text"],
            "left-ok"
        );
        assert_eq!(
            joined["checkpoint"]["nodes"]["right"]["output"]["text"],
            "right-ok"
        );
        assert_eq!(
            joined["checkpoint"]["nodes"]["join"]["output"]["text"],
            "joined"
        );
    }

    #[tokio::test]
    async fn product_route_partial_and_running_crash_do_not_pass() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let llm = std::sync::Arc::new(ScriptedLlm::new(&["first-ok", "second-ok"]));
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks.clone());
        opts.seeded_runtime = Some(anycode_agent::AgentRuntime::sandboxed_scripted(
            llm,
            std::collections::HashMap::new(),
        ));
        let app = app_for_test_custom(&db_path, opts).await.unwrap();
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let proj = tmp.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        let (project, session) = seed_project(&db, &proj, "Partial").await;

        let (started_status, started) = json_req(
            app.clone(),
            "POST",
            &format!("/api/projects/{}/harness/graphs/start", project.id),
            json!({"graph": two_work_graph(), "session_id": session.id}),
        )
        .await;
        assert_eq!(started_status, StatusCode::OK, "{started}");
        assert!(
            started["status"]
                .as_str()
                .unwrap_or("")
                .contains("completed"),
            "{started}"
        );
        let run_id = started["run_id"].as_str().expect("run_id").to_string();
        let ck_path = tasks
            .join("harness/graphs")
            .join(&project.id)
            .join(&run_id)
            .join("checkpoint.json");

        let mut checkpoint: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&ck_path).unwrap()).unwrap();
        checkpoint["nodes"]["first"]["status"] = json!("partial");
        checkpoint["nodes"]["first"]["error"] = json!("remaining work");
        checkpoint["nodes"]["second"]["status"] = json!("pending");
        checkpoint["nodes"]["second"]["output"] = json!(null);
        std::fs::write(&ck_path, serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let (partial_status, partial) = json_req(
            app.clone(),
            "POST",
            &format!(
                "/api/projects/{}/harness/graphs/{run_id}/resume",
                project.id
            ),
            json!({"session_id": session.id}),
        )
        .await;
        assert_eq!(partial_status, StatusCode::OK, "{partial}");
        assert!(
            partial["status"].as_str().unwrap_or("").contains("partial"),
            "Partial must not pass downstream: {partial}"
        );
        assert_eq!(
            partial["checkpoint"]["nodes"]["second"]["status"],
            "pending"
        );

        checkpoint["nodes"]["first"]["status"] = json!("running");
        checkpoint["nodes"]["first"]["attempts"] = json!(1);
        checkpoint["nodes"]["second"]["status"] = json!("pending");
        std::fs::write(&ck_path, serde_json::to_vec(&checkpoint).unwrap()).unwrap();
        let (crash_status, crashed) = json_req(
            app,
            "POST",
            &format!(
                "/api/projects/{}/harness/graphs/{run_id}/resume",
                project.id
            ),
            json!({"session_id": session.id}),
        )
        .await;
        assert_eq!(crash_status, StatusCode::OK, "{crashed}");
        assert!(
            crashed["status"]
                .as_str()
                .unwrap_or("")
                .contains("uncertain"),
            "Running after crash must become Uncertain: {crashed}"
        );
        assert_ne!(
            crashed["checkpoint"]["nodes"]["second"]["status"],
            "completed"
        );
    }

    #[tokio::test]
    async fn gray_allowlist_denies_other_projects() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let (project, session) = seed_project(&db, &tmp.path().join("proj"), "Gray").await;
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.harness_gray_projects = vec!["someone-else".into()];
        opts.tasks_root = Some(tasks);
        let app = app_for_test_custom(&db_path, opts).await.unwrap();
        let (status, body) = json_req(
            app,
            "POST",
            &format!("/api/projects/{}/harness/graphs/start", project.id),
            json!({"graph": human_graph(), "session_id": session.id}),
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}");
    }

    #[tokio::test]
    async fn local_acl_refuses_enterprise_without_live_grant_versions() {
        let project = Uuid::new_v4();
        let identity = Identity {
            active: true,
            iss: "anycode.local".into(),
            sub: Uuid::new_v4(),
            aud: "anycode".into(),
            context: ContextKind::Enterprise,
            tenant: Some(anycode_harness_cloud818::identity::TenantGrant {
                organization_id: Uuid::new_v4(),
                tenant_id: Uuid::new_v4(),
                external_tenant_id: "ext".into(),
                role: anycode_harness_cloud818::identity::Role::Editor,
                organization_version: 1,
                member_version: 1,
                tenant_version: 1,
                grant_version: 1,
            }),
            scope: "identity".into(),
        };
        let acl = LocalWorkbenchAcl {
            project,
            project_id: "proj_test".into(),
            db: None,
            require_accounts_membership: false,
        };
        assert!(acl.authorize_project(&identity, project).await.is_err());
    }

    #[test]
    fn desktop_never_attaches_sso_client() {
        assert!(crate::api::state::accounts_client_if_server_side(true).is_none());
    }

    fn loopback_accounts_client() -> (
        std::sync::Arc<anycode_harness_cloud818::identity::AccountsClient>,
        String,
        Uuid,
    ) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let issuer = format!("http://127.0.0.1:{port}");
        let subject = Uuid::new_v4();
        let body = serde_json::json!({
            "active": true,
            "iss": issuer,
            "sub": subject,
            "aud": "anycode",
            "context": "personal",
            "tenant": null,
            "scope": "identity"
        })
        .to_string();
        std::thread::spawn(move || {
            for _ in 0..8 {
                let Ok((mut stream, _)) = listener.accept() else {
                    break;
                };
                let mut req = [0u8; 4096];
                let _ = std::io::Read::read(&mut stream, &mut req);
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
            }
        });
        let client = anycode_harness_cloud818::identity::AccountsClient::new(
            &format!("{issuer}/"),
            "s".repeat(32),
            true,
        )
        .unwrap();
        (std::sync::Arc::new(client), "A".repeat(43), subject)
    }

    #[tokio::test]
    async fn graph_start_uses_authorize_token_when_sso_client_attached() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let (project, session) = seed_project(&db, &tmp.path().join("proj"), "Sso").await;
        let (client, token, subject) = loopback_accounts_client();
        db.grant_harness_accounts_member(&project.id, &subject.to_string())
            .await
            .unwrap();
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks);
        opts.accounts_sso = Some(client);
        let app = app_for_test_custom(&db_path, opts).await.unwrap();

        let status_res = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/harness/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let status_json: serde_json::Value =
            serde_json::from_slice(&status_res.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(status_json["accounts_sso"], true);
        assert_eq!(status_json["pairing"], "accounts_sso");

        let missing = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/api/projects/{}/harness/graphs/start", project.id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "graph": human_graph(),
                            "session_id": session.id
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);

        let ok = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/api/projects/{}/harness/graphs/start", project.id))
                    .header("content-type", "application/json")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "graph": human_graph(),
                            "session_id": session.id
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK, "introspect then ProductAcl");
    }

    #[tokio::test]
    async fn graph_start_sso_without_membership_is_forbidden() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let (project, session) = seed_project(&db, &tmp.path().join("proj"), "NoMember").await;
        let (client, token, _subject) = loopback_accounts_client();
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks);
        opts.accounts_sso = Some(client);
        let app = app_for_test_custom(&db_path, opts).await.unwrap();
        let denied = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/api/projects/{}/harness/graphs/start", project.id))
                    .header("content-type", "application/json")
                    .header("Authorization", format!("Bearer {token}"))
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "graph": human_graph(),
                            "session_id": session.id
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            denied.status(),
            StatusCode::FORBIDDEN,
            "SSO success without membership must not authorize"
        );
    }

    #[tokio::test]
    async fn pairing_confirm_then_revoke_denies_device_header() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let (project, session) = seed_project(&db, &tmp.path().join("proj"), "Pair").await;
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks);
        let app = app_for_test_custom(&db_path, opts).await.unwrap();

        let (st, challenge_body) = json_req(
            app.clone(),
            "POST",
            "/api/harness/pairing/challenges",
            json!({"label":"test-mac"}),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{challenge_body}");
        assert!(challenge_body["note"]
            .as_str()
            .unwrap_or("")
            .contains("PRODUCT_SSO_CLIENT_SECRET"));
        let challenge = challenge_body["challenge"].as_str().unwrap().to_string();

        let (st, confirm) = json_req(
            app.clone(),
            "POST",
            "/api/harness/pairing/confirm",
            json!({ "challenge": challenge }),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{confirm}");
        let device_id = confirm["device_id"].as_str().unwrap().to_string();
        let device_token = confirm["token"].as_str().unwrap().to_string();
        assert!(anycode_apple_media::is_harness_device_account(&device_id));
        assert!(anycode_apple_media::is_harness_device_token(&device_token));
        assert_eq!(confirm["keychain_service"], "anycode.harness.device");

        let (st, replay) = json_req(
            app.clone(),
            "POST",
            "/api/harness/pairing/confirm",
            json!({ "challenge": challenge }),
        )
        .await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{replay}");

        let ok = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/api/projects/{}/harness/graphs/start", project.id))
                    .header("content-type", "application/json")
                    .header("x-anycode-device-token", &device_token)
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "graph": human_graph(),
                            "session_id": session.id
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ok.status(), StatusCode::OK, "active device may start");

        let (st, revoked) = json_req(
            app.clone(),
            "POST",
            "/api/harness/pairing/revoke",
            json!({ "device_id": device_id }),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{revoked}");

        let denied = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/api/projects/{}/harness/graphs/start", project.id))
                    .header("content-type", "application/json")
                    .header("x-anycode-device-token", &device_token)
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "graph": human_graph(),
                            "session_id": session.id
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            denied.status(),
            StatusCode::FORBIDDEN,
            "revoked device must not authorize"
        );
    }

    #[tokio::test]
    async fn computer_ticket_binds_to_in_flight_run_after_enroll() {
        let tmp = tempfile::tempdir().unwrap();
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let shots = tmp.path().join("shots");
        std::fs::create_dir_all(&shots).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let (project, session) = seed_project(&db, &tmp.path().join("proj"), "Computer").await;
        let runtime = anycode_agent::AgentRuntime::sandboxed_scripted(
            std::sync::Arc::new(ScriptedLlm::new(&["waiting"])),
            std::collections::HashMap::new(),
        );
        runtime.attach_harness_tools().await;
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks);
        opts.seeded_runtime = Some(runtime.clone());
        let app = app_for_test_custom(&db_path, opts).await.unwrap();

        let (st, challenge_body) = json_req(
            app.clone(),
            "POST",
            "/api/harness/pairing/challenges",
            json!({"label":"computer-mac"}),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{challenge_body}");
        let (st, confirm) = json_req(
            app.clone(),
            "POST",
            "/api/harness/pairing/confirm",
            json!({ "challenge": challenge_body["challenge"] }),
        )
        .await;
        assert_eq!(st, StatusCode::OK, "{confirm}");
        let device_id = confirm["device_id"].as_str().unwrap();
        let device = Uuid::parse_str(device_id).unwrap();
        let device_token = confirm["token"].as_str().unwrap().to_string();
        runtime
            .enroll_macos_screencapture(device, &shots)
            .expect("enroll macOS observe backend");

        let started = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/api/projects/{}/harness/graphs/start", project.id))
                    .header("content-type", "application/json")
                    .header("x-anycode-device-token", &device_token)
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "graph": human_graph(),
                            "session_id": session.id
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(started.status(), StatusCode::OK);
        let started_json: serde_json::Value =
            serde_json::from_slice(&started.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        let run_id = started_json["run_id"].as_str().expect("run_id");

        let ticket_res = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/projects/{}/harness/computer/tickets",
                        project.id
                    ))
                    .header("content-type", "application/json")
                    .header("x-anycode-device-token", &device_token)
                    .body(Body::from(
                        serde_json::to_vec(&json!({ "run_id": run_id, "ttl_secs": 60 })).unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ticket_res.status(), StatusCode::OK);
        let ticket: serde_json::Value =
            serde_json::from_slice(&ticket_res.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert_eq!(ticket["run_id"], run_id);
        assert_eq!(ticket["device"], device_id);
        assert_eq!(ticket["backend"], "macos-screencapture");
        assert_eq!(ticket["x11_verified"], false);
        let ticket_id = ticket["ticket"].as_str().expect("ticket");
        let observe_ctx = RunContext::root(
            anycode_harness_core::Scope {
                subject: Uuid::new_v4(),
                organization: None,
                tenant: None,
                project: Uuid::new_v4(),
                device: Some(device),
            },
            anycode_harness_core::Capabilities::new(["computer.observe".into()]).unwrap(),
            BudgetPool::new(10_000).unwrap(),
            Duration::from_secs(8),
        )
        .unwrap();
        let _guard = runtime.enter_harness_run(observe_ctx, &shots);
        let observed = runtime
            .host_execute_tool_call(
                Uuid::new_v4(),
                &anycode_core::AgentType::new("general-purpose"),
                shots.to_str().unwrap(),
                &anycode_core::ToolCall {
                    id: "obs-product".into(),
                    name: "HarnessComputerObserve".into(),
                    input: json!({ "ticket": ticket_id }),
                },
            )
            .await
            .expect("product ticket must observe through the enrolled backend");
        assert_eq!(observed.result["backend"], "macos-screencapture");
        assert_eq!(observed.result["private"], true);
        let path = observed.result["path"].as_str().expect("path");
        let bytes = std::fs::read(path).expect("private png");
        assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
        assert!(bytes.len() > 64);
    }

    #[tokio::test]
    #[ignore = "paid LLM; export ANYCODE_HARNESS_LIVE_LLM=1; --test-threads=1; does not print secrets"]
    async fn live_product_graph_fileread_uses_configured_provider() {
        assert_eq!(
            std::env::var("ANYCODE_HARNESS_LIVE_LLM").ok().as_deref(),
            Some("1"),
            "refusing to call a paid provider unless ANYCODE_HARNESS_LIVE_LLM=1"
        );
        const MARKER: &str = "HARNESS_LIVE_LLM_MARKER_7c2e";
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("MARKER.txt"), MARKER).unwrap();
        let mut config = anycode_config::load_runtime_config(anycode_config::LoadOpts {
            config_file: None,
            ignore_approval: true,
            workspace_overlay: false,
            workspace_overlay_dir: Some(tmp.path().to_path_buf()),
        })
        .await
        .expect("load runtime config");
        if anycode_llm::normalize_provider_id(&config.llm.provider) == "anycode_cloud"
            && anycode_llm::refresh_cloud_access_token().await.is_err()
        {
            if let Some(key) = config
                .llm
                .provider_credentials
                .get("deepseek")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
            {
                config.llm.provider = "deepseek".into();
                config.llm.api_key = key;
                config.llm.model = "deepseek-v4-flash".into();
                config.llm.base_url =
                    anycode_llm::suggested_openai_base_for("deepseek").map(str::to_string);
            }
        }
        let runtime = anycode_bootstrap::initialize_runtime(
            &config,
            anycode_bootstrap::RuntimeHosts::default(),
            anycode_bootstrap::MemoryAttachMode::Shared,
            None,
            Some(tmp.path()),
        )
        .await
        .expect("initialize_runtime");
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks);
        opts.seeded_runtime = Some(runtime);
        let app = app_for_test_custom(&db_path, opts).await.unwrap();
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let (project, session) = seed_project(&db, tmp.path(), "LiveGraph").await;
        let (status, body) = json_req(
            app,
            "POST",
            &format!("/api/projects/{}/harness/graphs/start", project.id),
            json!({
                "graph": {
                    "version": 1,
                    "name": "live-fileread",
                    "nodes": [{
                        "id": "read",
                        "kind": {
                            "type": "work",
                            "agent": "explore",
                            "prompt": "Read MARKER.txt with FileRead. Quote the exact file contents. Do not guess."
                        }
                    }]
                },
                "session_id": session.id
            }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let blob = body.to_string();
        assert!(
            blob.contains(MARKER),
            "product GraphRunner must return the real file marker through Kernel"
        );
    }

    fn lx_account_cookie(set_cookie: &str) -> Option<String> {
        set_cookie
            .split(';')
            .next()
            .filter(|c| c.starts_with("lx_account="))
            .map(str::to_string)
    }

    async fn mint_live_sso_token(issuer: &str, secret: &str, redirect: &str) -> String {
        use base64::Engine;
        let origin = issuer.trim_end_matches('/');
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap();
        let login = http
            .post(format!("{origin}/api/v1/auth/otp/login"))
            .json(&json!({"phone":"13800138000","code":"000000"}))
            .send()
            .await
            .expect("otp login");
        assert!(login.status().is_success(), "otp login {}", login.status());
        let cookie = login
            .headers()
            .get_all(reqwest::header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .find_map(lx_account_cookie)
            .expect("lx_account cookie");
        let verifier = format!("Vv-_{}", "a".repeat(39));
        let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(sha2::Sha256::digest(verifier.as_bytes()));
        let authorize = http
            .get(format!("{origin}/api/v2/sso/authorize"))
            .query(&[
                ("client_id", "anycode"),
                ("redirect_uri", redirect),
                ("state", "harness-live-graph"),
                ("code_challenge", challenge.as_str()),
                ("code_challenge_method", "S256"),
            ])
            .header(reqwest::header::COOKIE, &cookie)
            .send()
            .await
            .expect("authorize");
        assert_eq!(authorize.status(), reqwest::StatusCode::SEE_OTHER);
        let loc = authorize
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
            .expect("redirect");
        let code = reqwest::Url::parse(loc)
            .expect("callback")
            .query_pairs()
            .find(|(k, _)| k == "code")
            .map(|(_, v)| v.into_owned())
            .expect("code");
        let token_res = http
            .post(format!("{origin}/api/v2/sso/token"))
            .basic_auth("anycode", Some(secret))
            .json(&json!({
                "code": code,
                "redirect_uri": redirect,
                "code_verifier": verifier
            }))
            .send()
            .await
            .expect("token");
        assert!(
            token_res.status().is_success(),
            "token {}",
            token_res.status()
        );
        token_res.json::<serde_json::Value>().await.unwrap()["access_token"]
            .as_str()
            .expect("access_token")
            .to_string()
    }

    #[tokio::test]
    #[ignore = "live 818cloud + paid LLM; export ANYCODE_HARNESS_LIVE_LLM=1 and SSO issuer/secret; --test-threads=1; does not print secrets"]
    async fn live_sso_pairing_keychain_and_graph_fileread() {
        assert_eq!(
            std::env::var("ANYCODE_HARNESS_LIVE_LLM").ok().as_deref(),
            Some("1")
        );
        let issuer = std::env::var("ANYCODE_HARNESS_SSO_ISSUER").expect("issuer");
        let secret = std::env::var("PRODUCT_SSO_CLIENT_SECRET").expect("secret");
        let redirect = std::env::var("ANYCODE_HARNESS_SSO_REDIRECT")
            .unwrap_or_else(|_| "http://127.0.0.1:18781/api/auth/hop/v2/callback".into());
        let loopback =
            issuer.starts_with("http://127.0.0.1") || issuer.starts_with("http://localhost");
        let token = mint_live_sso_token(&issuer, &secret, &redirect).await;
        let client = std::sync::Arc::new(
            anycode_harness_cloud818::identity::AccountsClient::new(&issuer, secret, loopback)
                .expect("accounts client"),
        );
        let identity = client.introspect(&token, None).await.expect("introspect");
        assert!(identity.active);

        const MARKER: &str = "HARNESS_LIVE_LLM_MARKER_7c2e";
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("MARKER.txt"), MARKER).unwrap();
        let mut config = anycode_config::load_runtime_config(anycode_config::LoadOpts {
            config_file: None,
            ignore_approval: true,
            workspace_overlay: false,
            workspace_overlay_dir: Some(tmp.path().to_path_buf()),
        })
        .await
        .expect("load runtime config");
        if anycode_llm::normalize_provider_id(&config.llm.provider) == "anycode_cloud"
            && anycode_llm::refresh_cloud_access_token().await.is_err()
        {
            if let Some(key) = config
                .llm
                .provider_credentials
                .get("deepseek")
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
            {
                config.llm.provider = "deepseek".into();
                config.llm.api_key = key;
                config.llm.model = "deepseek-v4-flash".into();
                config.llm.base_url =
                    anycode_llm::suggested_openai_base_for("deepseek").map(str::to_string);
            }
        }
        let runtime = anycode_bootstrap::initialize_runtime(
            &config,
            anycode_bootstrap::RuntimeHosts::default(),
            anycode_bootstrap::MemoryAttachMode::Shared,
            None,
            Some(tmp.path()),
        )
        .await
        .expect("initialize_runtime");
        let tasks = tmp.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        let db_path = tmp.path().join("db.sqlite");
        let mut opts = TestAppOptions::default();
        opts.harness_graph = true;
        opts.tasks_root = Some(tasks);
        opts.seeded_runtime = Some(runtime);
        opts.accounts_sso = Some(client);
        let app = app_for_test_custom(&db_path, opts).await.unwrap();
        let db = crate::db::DashboardDb::open(&db_path).await.unwrap();
        let (project, session) = seed_project(&db, tmp.path(), "LiveSsoGraph").await;
        db.grant_harness_accounts_member(&project.id, &identity.sub.to_string())
            .await
            .unwrap();

        let challenge_req = axum::http::Request::builder()
            .method("POST")
            .uri("/api/harness/pairing/challenges")
            .header("content-type", "application/json")
            .header("authorization", format!("Bearer {token}"))
            .body(Body::from(
                serde_json::to_vec(&json!({"label":"live-mac"})).unwrap(),
            ))
            .unwrap();
        let challenge_res = app.clone().oneshot(challenge_req).await.unwrap();
        assert_eq!(challenge_res.status(), StatusCode::OK);
        let challenge_json: serde_json::Value = serde_json::from_slice(
            &challenge_res
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes(),
        )
        .unwrap();
        let confirm_res = app
            .clone()
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri("/api/harness/pairing/confirm")
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "challenge": challenge_json["challenge"]
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(confirm_res.status(), StatusCode::OK);
        let confirm: serde_json::Value =
            serde_json::from_slice(&confirm_res.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        let device_id = confirm["device_id"]
            .as_str()
            .expect("device_id")
            .to_string();
        let device_token = confirm["token"].as_str().expect("token").to_string();
        assert!(anycode_apple_media::is_harness_device_account(&device_id));
        assert!(anycode_apple_media::is_harness_device_token(&device_token));
        assert_eq!(confirm["keychain_service"], "anycode.harness.device");

        let helper = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../apps/anycode-desktop/resources/bin/anycode-apple-media");
        assert!(
            helper.is_file(),
            "Desktop helper is required to persist pairing tokens"
        );
        anycode_apple_media::harness_device_token_set(&[helper.clone()], &device_id, &device_token)
            .expect("persist pairing token via the Desktop keychain contract");
        let stored = anycode_apple_media::harness_device_token_get(&[helper], &device_id)
            .expect("read pairing token via the Desktop keychain contract");
        assert_eq!(stored.as_deref(), Some(device_token.as_str()));
        let _ = std::process::Command::new("security")
            .args([
                "delete-generic-password",
                "-s",
                "anycode.harness.device",
                "-a",
                &device_id,
            ])
            .status();

        let started = app
            .oneshot(
                axum::http::Request::builder()
                    .method("POST")
                    .uri(format!("/api/projects/{}/harness/graphs/start", project.id))
                    .header("content-type", "application/json")
                    .header("authorization", format!("Bearer {token}"))
                    .header("x-anycode-device-token", &device_token)
                    .body(Body::from(
                        serde_json::to_vec(&json!({
                            "graph": {
                                "version": 1,
                                "name": "live-sso-fileread",
                                "nodes": [{
                                    "id": "read",
                                    "kind": {
                                        "type": "work",
                                        "agent": "explore",
                                        "prompt": "Read MARKER.txt with FileRead. Quote the exact file contents. Do not guess."
                                    }
                                }]
                            },
                            "session_id": session.id
                        }))
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(started.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&started.into_body().collect().await.unwrap().to_bytes())
                .unwrap();
        assert!(
            body.to_string().contains(MARKER),
            "818cloud + pairing + GraphRunner must return the real file marker"
        );
    }
}
