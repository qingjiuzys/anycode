//! Paid live Kernel FileRead. Default CI does not run this.
//! Does not print config, keys, or provider payloads.
use anycode_agent::{HarnessHostPolicy, ReadOnlyPilotBoundary, RunLifecycle};
use anycode_bootstrap::{initialize_runtime, MemoryAttachMode, RuntimeHosts};
use anycode_config::{load_runtime_config, LoadOpts};
use anycode_core::{
    AgentLoopLimits, AgentType, FeatureFlag, Task, TaskBudget, TaskContext, TaskResult,
};
use anycode_harness_core::{
    budget::BudgetPool,
    events::PreviewBus,
    journal::MemoryJournal,
    kernel::{ControlQueue, Host, Kernel},
    types::Limits,
    Capabilities, RunContext, Scope,
};
use sha2::Digest;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

const MARKER: &str = "HARNESS_LIVE_LLM_MARKER_7c2e";
const FILE_READ_PROMPT: &str =
    "Read MARKER.txt with FileRead. Quote the exact file contents. Do not guess.";

fn require_paid_live_llm() {
    assert_eq!(
        std::env::var("ANYCODE_HARNESS_LIVE_LLM").ok().as_deref(),
        Some("1"),
        "refusing to call a paid provider unless ANYCODE_HARNESS_LIVE_LLM=1"
    );
}

fn live_marker_dir() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("MARKER.txt"), MARKER).unwrap();
    tmp
}

fn live_budget() -> TaskBudget {
    TaskBudget {
        token_budget_total: Some(200_000),
        max_duration_secs: Some(90),
        ..TaskBudget::default()
    }
}

async fn load_armed_config(workspace: &std::path::Path) -> anycode_config::Config {
    let mut config = load_runtime_config(LoadOpts {
        config_file: None,
        ignore_approval: true,
        workspace_overlay: false,
        workspace_overlay_dir: Some(workspace.to_path_buf()),
    })
    .await
    .expect("load runtime config");
    arm_live_provider(&mut config).await;
    config
}

async fn init_live_runtime(
    config: &anycode_config::Config,
    workspace: &std::path::Path,
) -> Arc<anycode_agent::AgentRuntime> {
    initialize_runtime(
        config,
        RuntimeHosts::default(),
        MemoryAttachMode::Shared,
        None,
        Some(workspace),
    )
    .await
    .expect("initialize_runtime")
}

/// Prefer the product anycode_cloud session. If device refresh is dead, use a
/// configured BYOK provider in memory. Never prints keys or URLs with tokens.
async fn arm_live_provider(config: &mut anycode_config::Config) {
    if anycode_llm::normalize_provider_id(&config.llm.provider) == "anycode_cloud" {
        if anycode_llm::refresh_cloud_access_token().await.is_ok() {
            return;
        }
        const FALLBACKS: &[(&str, &str)] = &[
            ("deepseek", "deepseek-v4-flash"),
            ("anthropic", "claude-sonnet-4-20250514"),
            ("alibaba", "qwen-plus"),
        ];
        for (id, model) in FALLBACKS {
            let Some(key) = config
                .llm
                .provider_credentials
                .get(*id)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            config.llm.provider = (*id).to_string();
            config.llm.api_key = key;
            config.llm.model = (*model).to_string();
            config.llm.base_url = anycode_llm::suggested_openai_base_for(id).map(str::to_string);
            return;
        }
    }
    assert!(
        !config.llm.model.trim().is_empty(),
        "configured model id is required"
    );
    assert!(
        !config.llm.provider.trim().is_empty(),
        "configured provider id is required"
    );
}

#[tokio::test]
#[ignore = "paid LLM; export ANYCODE_HARNESS_LIVE_LLM=1; reads ~/.anycode/config.json and does not print secrets"]
async fn live_kernel_fileread_uses_configured_provider() {
    require_paid_live_llm();
    let tmp = live_marker_dir();
    let config = load_armed_config(tmp.path()).await;
    let runtime = init_live_runtime(&config, tmp.path()).await;
    let ctx = RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new(["fs.read".into()]).unwrap(),
        BudgetPool::new(200_000).unwrap(),
        Duration::from_secs(90),
    )
    .unwrap();
    let mut bindings = BTreeMap::new();
    bindings.insert("FileRead".into(), ("fs.read".into(), true));
    let lifecycle = Arc::new(RunLifecycle::default());
    let boundary = Arc::new(ReadOnlyPilotBoundary::new(&ctx, tmp.path()).unwrap());
    let host = runtime
        .harness_host(
            &ctx,
            tmp.path(),
            AgentType::new("explore"),
            runtime.harness_model_config(),
            bindings,
            boundary,
            PreviewBus::default(),
            lifecycle,
            HarnessHostPolicy::readonly_graph(),
        )
        .await
        .expect("harness host");
    let journal = MemoryJournal::default();
    let queue = ControlQueue::default();
    let kernel = Kernel {
        host: &host,
        journal: &journal,
        previews: PreviewBus::default(),
        controls: &queue,
        limits: Limits {
            max_turns: 4,
            reservation_per_hop: 32_768,
            ..Limits::default()
        },
    };
    let messages = kernel
        .run(&ctx, vec![host.user_message(FILE_READ_PROMPT).unwrap()])
        .await
        .expect("live Kernel FileRead");
    let blob = serde_json::to_string(&messages).expect("messages json");
    assert!(
        blob.contains(MARKER),
        "provider must return the real file marker through Kernel"
    );
}

#[tokio::test]
#[ignore = "paid LLM; export ANYCODE_HARNESS_LIVE_LLM=1; --test-threads=1; does not print secrets"]
async fn live_execute_task_unified_kernel_fileread_uses_configured_provider() {
    require_paid_live_llm();
    let tmp = live_marker_dir();
    let mut config = load_armed_config(tmp.path()).await;
    config
        .runtime
        .features
        .enable(FeatureFlag::HarnessV1UnifiedKernel.as_str());
    let runtime = init_live_runtime(&config, tmp.path()).await;
    assert!(
        runtime.harness_unified_kernel_enabled(),
        "in-memory feature must arm the scheduler-equivalent execute_task adapter"
    );
    let result = runtime
        .execute_task(Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("explore"),
            prompt: FILE_READ_PROMPT.to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: tmp.path().display().to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
                channel_progress_tx: None,
                live_trace_tx: None,
                tool_deny_names: vec![],
                tool_deny_prefixes: vec![],
                user_vision_images: vec![],
                budget: live_budget(),
                loop_limits: AgentLoopLimits::clamped(4, 4),
                chat_turn: None,
            },
            created_at: chrono::Utc::now(),
        })
        .await
        .expect("live execute_task FileRead");
    match result {
        TaskResult::Success { output, .. } => {
            assert!(
                output.contains(MARKER),
                "unified execute_task must return the real file marker"
            );
        }
        other => panic!("expected Success with file marker, got {other:?}"),
    }
}

#[tokio::test]
#[ignore = "paid LLM; export ANYCODE_HARNESS_LIVE_LLM=1; --test-threads=1; does not print secrets"]
async fn live_execute_turn_unified_kernel_fileread_uses_configured_provider() {
    require_paid_live_llm();
    let tmp = live_marker_dir();
    let mut config = load_armed_config(tmp.path()).await;
    config
        .runtime
        .features
        .enable(FeatureFlag::HarnessV1UnifiedKernel.as_str());
    let runtime = init_live_runtime(&config, tmp.path()).await;
    assert!(
        runtime.harness_unified_kernel_enabled(),
        "in-memory feature must arm the Workbench execute_turn adapter"
    );
    let agent = AgentType::new("explore");
    let wd = tmp.path().to_str().unwrap();
    let mut history = vec![runtime
        .build_system_message(&agent, wd)
        .await
        .expect("system message")];
    history.push(anycode_core::Message {
        id: Uuid::new_v4(),
        role: anycode_core::MessageRole::User,
        content: anycode_core::MessageContent::Text(FILE_READ_PROMPT.into()),
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    });
    let messages = Arc::new(tokio::sync::Mutex::new(history));
    let out = runtime
        .execute_turn_from_messages(
            Uuid::new_v4(),
            &agent,
            messages.clone(),
            wd,
            None,
            &[],
            &[],
            live_budget(),
            AgentLoopLimits::clamped(4, 4),
            None,
        )
        .await
        .expect("live execute_turn FileRead");
    let blob = serde_json::to_string(&*messages.lock().await).expect("messages json");
    assert!(
        blob.contains(MARKER) || out.final_text.contains(MARKER),
        "unified execute_turn must return the real file marker"
    );
}

#[tokio::test]
#[ignore = "paid LLM; export ANYCODE_HARNESS_LIVE_LLM=1; --test-threads=1; does not print secrets"]
async fn live_workbench_chat_runtime_fileread_uses_configured_provider() {
    require_paid_live_llm();
    let tmp = live_marker_dir();
    let mut config = load_armed_config(tmp.path()).await;
    config
        .runtime
        .features
        .enable(FeatureFlag::HarnessV1UnifiedKernel.as_str());
    let runtime = init_live_runtime(&config, tmp.path()).await;
    let db = anycode_dashboard::DashboardDb::open(&tmp.path().join("db.sqlite"))
        .await
        .expect("db");
    let project = db
        .upsert_project(anycode_dashboard::schema::UpsertProjectRequest {
            root_path: tmp.path().to_string_lossy().into(),
            name: Some("LiveWorkbench".into()),
            create_root: Some(true),
            ..Default::default()
        })
        .await
        .expect("project");
    let session = db
        .create_session(anycode_dashboard::schema::CreateSessionRequest {
            project_id: project.id.clone(),
            kind: "repl".into(),
            task_id: None,
            title: "LiveWorkbench".into(),
            prompt_preview: None,
            agent_type: Some("explore".into()),
            model: None,
            metadata_json: None,
        })
        .await
        .expect("session");
    let host = anycode_dashboard::control::chat_runtime::ChatRuntimeHost::new()
        .with_seeded_runtime(runtime);
    let events = Arc::new(anycode_dashboard::EventBus::new());
    let tail = anycode_dashboard::control::web_chat_tail::WebChatTailHub::default();
    let prompt = FILE_READ_PROMPT;
    host.send(
        db.clone(),
        events,
        &tail,
        &session.id,
        &project.id,
        tmp.path(),
        Some("explore"),
        prompt,
        prompt,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("workbench send");
    wait_for_marker_in_session(&db, &session.id).await;
}

#[tokio::test]
#[ignore = "paid LLM; export ANYCODE_HARNESS_LIVE_LLM=1; --test-threads=1; does not print secrets"]
async fn live_supervisor_child_fileread_uses_configured_provider() {
    require_paid_live_llm();
    let tmp = live_marker_dir();
    let mut config = load_armed_config(tmp.path()).await;
    config
        .runtime
        .features
        .enable(FeatureFlag::HarnessV1UnifiedKernel.as_str());
    let runtime = init_live_runtime(&config, tmp.path()).await;
    let ctx = RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new(["agent.spawn".into(), "fs.read".into()]).unwrap(),
        BudgetPool::new(200_000).unwrap(),
        Duration::from_secs(90),
    )
    .unwrap();
    let _guard = runtime.enter_harness_run(ctx, tmp.path());
    let wd = tmp.path().to_str().unwrap();
    let spawned = host_execute_named(
        &runtime,
        wd,
        "live-spawn",
        "HarnessAgentSpawn",
        serde_json::json!({
            "agent": "explore",
            "prompt": FILE_READ_PROMPT,
            "capabilities": ["fs.read"]
        }),
    )
    .await
    .expect("spawn live child");
    let child_run_id = spawned.result["child_run_id"].clone();
    let joined = host_execute_named(
        &runtime,
        wd,
        "live-join",
        "HarnessAgentJoin",
        serde_json::json!({ "child_run_id": child_run_id }),
    )
    .await
    .expect("join live child");
    let blob = serde_json::to_string(&joined.result).expect("child json");
    assert!(
        blob.contains(MARKER),
        "Supervisor child must return the real file marker through the same Kernel"
    );
}

#[tokio::test]
#[ignore = "paid LLM + live 818cloud; export ANYCODE_HARNESS_LIVE_LLM=1 and SSO issuer/secret; --test-threads=1; does not print secrets"]
async fn live_workbench_supervisor_and_graph_fileread_same_runtime() {
    require_paid_live_llm();
    let tmp = live_marker_dir();
    let mut config = load_armed_config(tmp.path()).await;
    config
        .runtime
        .features
        .enable(FeatureFlag::HarnessV1UnifiedKernel.as_str());
    let runtime = init_live_runtime(&config, tmp.path()).await;
    let db_path = tmp.path().join("db.sqlite");
    let db = anycode_dashboard::DashboardDb::open(&db_path)
        .await
        .expect("db");
    let project = db
        .upsert_project(anycode_dashboard::schema::UpsertProjectRequest {
            root_path: tmp.path().to_string_lossy().into(),
            name: Some("LiveCombined".into()),
            create_root: Some(true),
            ..Default::default()
        })
        .await
        .expect("project");
    let session = db
        .create_session(anycode_dashboard::schema::CreateSessionRequest {
            project_id: project.id.clone(),
            kind: "repl".into(),
            task_id: None,
            title: "LiveCombined".into(),
            prompt_preview: None,
            agent_type: Some("explore".into()),
            model: None,
            metadata_json: None,
        })
        .await
        .expect("session");

    let host = anycode_dashboard::control::chat_runtime::ChatRuntimeHost::new()
        .with_seeded_runtime(runtime.clone());
    let events = Arc::new(anycode_dashboard::EventBus::new());
    let tail = anycode_dashboard::control::web_chat_tail::WebChatTailHub::default();
    host.send(
        db.clone(),
        events,
        &tail,
        &session.id,
        &project.id,
        tmp.path(),
        Some("explore"),
        FILE_READ_PROMPT,
        FILE_READ_PROMPT,
        None,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("workbench send");
    wait_for_marker_in_session(&db, &session.id).await;

    let ctx = RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new(["agent.spawn".into(), "fs.read".into()]).unwrap(),
        BudgetPool::new(200_000).unwrap(),
        Duration::from_secs(90),
    )
    .unwrap();
    let _guard = runtime.enter_harness_run(ctx, tmp.path());
    let wd = tmp.path().to_str().unwrap();
    let spawned = host_execute_named(
        &runtime,
        wd,
        "combined-spawn",
        "HarnessAgentSpawn",
        serde_json::json!({
            "agent": "explore",
            "prompt": FILE_READ_PROMPT,
            "capabilities": ["fs.read"]
        }),
    )
    .await
    .expect("spawn combined child");
    let joined = host_execute_named(
        &runtime,
        wd,
        "combined-join",
        "HarnessAgentJoin",
        serde_json::json!({ "child_run_id": spawned.result["child_run_id"] }),
    )
    .await
    .expect("join combined child");
    drop(_guard);
    let child_blob = serde_json::to_string(&joined.result).expect("child json");
    assert!(
        child_blob.contains(MARKER),
        "Supervisor child in the combined run must return the file marker"
    );

    let scheduled = runtime
        .execute_task(Task {
            id: Uuid::new_v4(),
            agent_type: AgentType::new("explore"),
            prompt: FILE_READ_PROMPT.to_string(),
            context: TaskContext {
                session_id: Uuid::new_v4(),
                working_directory: tmp.path().display().to_string(),
                environment: HashMap::new(),
                user_id: None,
                system_prompt_append: None,
                context_injections: vec![],
                nested_model_override: None,
                nested_worktree_path: None,
                nested_worktree_repo_root: None,
                nested_cancel: None,
                channel_progress_tx: None,
                live_trace_tx: None,
                tool_deny_names: vec![],
                tool_deny_prefixes: vec![],
                user_vision_images: vec![],
                budget: live_budget(),
                loop_limits: AgentLoopLimits::clamped(4, 4),
                chat_turn: None,
            },
            created_at: chrono::Utc::now(),
        })
        .await
        .expect("scheduler-shaped execute_task");
    match scheduled {
        TaskResult::Success { output, .. } => {
            assert!(
                output.contains(MARKER),
                "scheduler-shaped hop in the combined run must return the file marker"
            );
        }
        other => panic!("expected scheduler Success with file marker, got {other:?}"),
    }

    let tasks = tmp.path().join("tasks");
    std::fs::create_dir_all(&tasks).unwrap();
    let mut opts = anycode_dashboard::server::TestAppOptions::default();
    opts.harness_graph = true;
    opts.tasks_root = Some(tasks.clone());
    opts.seeded_runtime = Some(runtime.clone());
    let app = anycode_dashboard::server::app_for_test_custom(&db_path, opts)
        .await
        .expect("test app");
    let (status, body) = graph_json_req(
        app,
        "POST",
        &format!("/api/projects/{}/harness/graphs/start", project.id),
        serde_json::json!({
            "graph": {
                "version": 1,
                "name": "live-combined-fileread",
                "nodes": [{
                    "id": "read",
                    "kind": {
                        "type": "work",
                        "agent": "explore",
                        "prompt": FILE_READ_PROMPT
                    }
                }]
            },
            "session_id": session.id
        }),
        None,
        None,
    )
    .await;
    assert_eq!(status, axum::http::StatusCode::OK, "{body}");
    assert!(
        body.to_string().contains(MARKER),
        "product GraphRunner in the combined run must return the file marker; status={} node_text={}",
        body.get("status").cloned().unwrap_or(serde_json::Value::Null),
        body.pointer("/checkpoint/nodes")
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    );

    let issuer = std::env::var("ANYCODE_HARNESS_SSO_ISSUER")
        .expect("combined M8 hop requires ANYCODE_HARNESS_SSO_ISSUER");
    let secret = std::env::var("PRODUCT_SSO_CLIENT_SECRET")
        .expect("combined M8 hop requires PRODUCT_SSO_CLIENT_SECRET");
    let redirect = std::env::var("ANYCODE_HARNESS_SSO_REDIRECT")
        .unwrap_or_else(|_| "http://127.0.0.1:18781/api/auth/hop/v2/callback".into());
    let loopback = issuer.starts_with("http://127.0.0.1") || issuer.starts_with("http://localhost");
    let token = mint_sso_access_token(&issuer, &secret, &redirect).await;
    let client = std::sync::Arc::new(
        anycode_harness_cloud818::identity::AccountsClient::new(&issuer, secret, loopback)
            .expect("accounts client"),
    );
    let identity = client.introspect(&token, None).await.expect("introspect");
    assert!(identity.active);
    db.grant_harness_accounts_member(&project.id, &identity.sub.to_string())
        .await
        .expect("grant member");
    let mut sso_opts = anycode_dashboard::server::TestAppOptions::default();
    sso_opts.harness_graph = true;
    sso_opts.tasks_root = Some(tasks);
    sso_opts.seeded_runtime = Some(runtime);
    sso_opts.accounts_sso = Some(client);
    let sso_app = anycode_dashboard::server::app_for_test_custom(&db_path, sso_opts)
        .await
        .expect("sso app");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind desktop pairing BFF");
    let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    tokio::spawn(async move {
        axum::serve(listener, sso_app).await.ok();
    });
    let challenge = anycode_harness_cloud818::desktop_pairing::post_pairing(
        &origin,
        "/api/harness/pairing/challenges",
        Some(&token),
        serde_json::json!({"label":"combined-desktop"}),
    )
    .await
    .expect("Desktop pairing client challenge");
    let confirm = anycode_harness_cloud818::desktop_pairing::post_pairing(
        &origin,
        "/api/harness/pairing/confirm",
        Some(&token),
        serde_json::json!({ "challenge": challenge["challenge"] }),
    )
    .await
    .expect("Desktop pairing client confirm");
    let device_id = confirm["device_id"].as_str().expect("device_id");
    let device_token = confirm["token"].as_str().expect("token");
    assert!(anycode_apple_media::is_harness_device_account(device_id));
    assert!(anycode_apple_media::is_harness_device_token(device_token));
    let helper = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/anycode-desktop/resources/bin/anycode-apple-media");
    assert!(
        helper.is_file(),
        "Desktop helper is required to persist pairing tokens"
    );
    anycode_apple_media::harness_device_token_set(&[helper.clone()], device_id, device_token)
        .expect("Desktop keychain persist");
    let stored = anycode_apple_media::harness_device_token_get(&[helper], device_id)
        .expect("Desktop keychain read");
    assert_eq!(stored.as_deref(), Some(device_token));
    let _ = std::process::Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            "anycode.harness.device",
            "-a",
            device_id,
        ])
        .status();
    let started = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(90))
        .build()
        .unwrap()
        .post(format!(
            "{origin}/api/projects/{}/harness/graphs/start",
            project.id
        ))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .header("x-anycode-device-token", device_token)
        .json(&serde_json::json!({
            "graph": {
                "version": 1,
                "name": "live-combined-sso-fileread",
                "nodes": [{
                    "id": "read",
                    "kind": {
                        "type": "work",
                        "agent": "explore",
                        "prompt": FILE_READ_PROMPT
                    }
                }]
            },
            "session_id": session.id
        }))
        .send()
        .await
        .expect("desktop-origin graph start");
    let st = started.status();
    let sso_body: serde_json::Value = started.json().await.unwrap_or(serde_json::json!({}));
    assert!(st.is_success(), "{st} {sso_body}");
    assert!(
        sso_body.to_string().contains(MARKER),
        "818cloud pairing + GraphRunner hop must return the file marker"
    );
}

#[tokio::test]
#[ignore = "paid LLM + live 818cloud PKCE hop; export ANYCODE_HARNESS_LIVE_LLM=1 and SSO issuer/secret; --test-threads=1"]
async fn live_sso_hop_then_desktop_pairing_fileread() {
    require_paid_live_llm();
    let issuer = std::env::var("ANYCODE_HARNESS_SSO_ISSUER").expect("issuer");
    let secret = std::env::var("PRODUCT_SSO_CLIENT_SECRET").expect("secret");
    let redirect = std::env::var("ANYCODE_HARNESS_SSO_REDIRECT")
        .unwrap_or_else(|_| "http://127.0.0.1:18781/api/auth/hop/v2/callback".into());
    let loopback = issuer.starts_with("http://127.0.0.1") || issuer.starts_with("http://localhost");
    let client = std::sync::Arc::new(
        anycode_harness_cloud818::identity::AccountsClient::new(&issuer, secret.clone(), loopback)
            .expect("accounts client"),
    );
    let tmp = live_marker_dir();
    let config = load_armed_config(tmp.path()).await;
    let runtime = init_live_runtime(&config, tmp.path()).await;
    let db_path = tmp.path().join("db.sqlite");
    let tasks = tmp.path().join("tasks");
    std::fs::create_dir_all(&tasks).unwrap();
    let db = anycode_dashboard::DashboardDb::open(&db_path)
        .await
        .expect("db");
    let (project, session) = {
        let project = db
            .upsert_project(anycode_dashboard::schema::UpsertProjectRequest {
                root_path: tmp.path().to_string_lossy().into(),
                name: Some("LiveSsoHop".into()),
                create_root: Some(true),
                ..Default::default()
            })
            .await
            .expect("project");
        let session = db
            .create_session(anycode_dashboard::schema::CreateSessionRequest {
                project_id: project.id.clone(),
                kind: "repl".into(),
                task_id: None,
                title: "LiveSsoHop".into(),
                prompt_preview: None,
                agent_type: Some("explore".into()),
                model: None,
                metadata_json: None,
            })
            .await
            .expect("session");
        (project, session)
    };
    let mut opts = anycode_dashboard::server::TestAppOptions::default();
    opts.harness_graph = true;
    opts.tasks_root = Some(tasks);
    opts.seeded_runtime = Some(runtime);
    opts.accounts_sso = Some(client.clone());
    opts.sso_redirect = Some(redirect.clone());
    let app = anycode_dashboard::server::app_for_test_custom(&db_path, opts)
        .await
        .expect("sso hop app");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind hop BFF");
    let origin = format!("http://127.0.0.1:{}", listener.local_addr().unwrap().port());
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let begin = anycode_harness_cloud818::desktop_pairing::post_pairing(
        &origin,
        "/api/harness/pairing/sso/begin",
        None,
        serde_json::json!({}),
    )
    .await
    .expect("sso begin");
    let authorize_url = begin["authorize_url"].as_str().expect("authorize_url");
    let poll_token = begin["poll_token"].as_str().expect("poll_token");
    let (code, state) = capture_sso_authorize_code(&issuer, authorize_url).await;
    anycode_harness_cloud818::desktop_pairing::post_pairing(
        &origin,
        "/api/harness/pairing/sso/complete",
        None,
        serde_json::json!({ "poll_token": poll_token, "code": code, "state": state }),
    )
    .await
    .expect("sso complete");
    let hop = anycode_harness_cloud818::desktop_pairing::post_pairing(
        &origin,
        "/api/harness/pairing/sso/poll",
        None,
        serde_json::json!({ "poll_token": poll_token }),
    )
    .await
    .expect("sso poll");
    let token = hop["access_token"]
        .as_str()
        .expect("access_token")
        .to_string();
    let identity = client
        .introspect(&token, None)
        .await
        .expect("introspect hop token");
    assert!(identity.active);
    db.grant_harness_accounts_member(&project.id, &identity.sub.to_string())
        .await
        .expect("grant");
    let challenge = anycode_harness_cloud818::desktop_pairing::post_pairing(
        &origin,
        "/api/harness/pairing/challenges",
        Some(&token),
        serde_json::json!({"label":"sso-hop-desktop"}),
    )
    .await
    .expect("challenge");
    let confirm = anycode_harness_cloud818::desktop_pairing::post_pairing(
        &origin,
        "/api/harness/pairing/confirm",
        Some(&token),
        serde_json::json!({ "challenge": challenge["challenge"] }),
    )
    .await
    .expect("confirm");
    let device_id = confirm["device_id"].as_str().expect("device_id");
    let device_token = confirm["token"].as_str().expect("token");
    let helper = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/anycode-desktop/resources/bin/anycode-apple-media");
    anycode_apple_media::harness_device_token_set(&[helper.clone()], device_id, device_token)
        .expect("keychain");
    let stored = anycode_apple_media::harness_device_token_get(&[helper], device_id).expect("read");
    assert_eq!(stored.as_deref(), Some(device_token));
    let _ = std::process::Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            "anycode.harness.device",
            "-a",
            device_id,
        ])
        .status();
    let started = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(90))
        .build()
        .unwrap()
        .post(format!(
            "{origin}/api/projects/{}/harness/graphs/start",
            project.id
        ))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .header("x-anycode-device-token", device_token)
        .json(&serde_json::json!({
            "graph": {
                "version": 1,
                "name": "live-sso-hop-fileread",
                "nodes": [{
                    "id": "read",
                    "kind": {
                        "type": "work",
                        "agent": "explore",
                        "prompt": FILE_READ_PROMPT
                    }
                }]
            },
            "session_id": session.id
        }))
        .send()
        .await
        .expect("graph start");
    let st = started.status();
    let body: serde_json::Value = started.json().await.unwrap_or(serde_json::json!({}));
    assert!(st.is_success(), "{st} {body}");
    assert!(
        body.to_string().contains(MARKER),
        "SSO hop + Desktop pairing + GraphRunner must return the file marker"
    );
}

#[tokio::test]
#[ignore = "paid LLM + live 818cloud; issuer must allow http://127.0.0.1:18782/api/harness/pairing/sso/callback"]
async fn live_sso_callback_landing_then_desktop_pairing_fileread() {
    require_paid_live_llm();
    let issuer = std::env::var("ANYCODE_HARNESS_SSO_ISSUER").expect("issuer");
    let secret = std::env::var("PRODUCT_SSO_CLIENT_SECRET").expect("secret");
    let redirect = "http://127.0.0.1:18782/api/harness/pairing/sso/callback";
    let loopback = issuer.starts_with("http://127.0.0.1") || issuer.starts_with("http://localhost");
    let client = std::sync::Arc::new(
        anycode_harness_cloud818::identity::AccountsClient::new(&issuer, secret.clone(), loopback)
            .expect("accounts client"),
    );
    let tmp = live_marker_dir();
    let config = load_armed_config(tmp.path()).await;
    let runtime = init_live_runtime(&config, tmp.path()).await;
    let db_path = tmp.path().join("db.sqlite");
    let tasks = tmp.path().join("tasks");
    std::fs::create_dir_all(&tasks).unwrap();
    let db = anycode_dashboard::DashboardDb::open(&db_path)
        .await
        .expect("db");
    let project = db
        .upsert_project(anycode_dashboard::schema::UpsertProjectRequest {
            root_path: tmp.path().to_string_lossy().into(),
            name: Some("LiveSsoCallback".into()),
            create_root: Some(true),
            ..Default::default()
        })
        .await
        .expect("project");
    let session = db
        .create_session(anycode_dashboard::schema::CreateSessionRequest {
            project_id: project.id.clone(),
            kind: "repl".into(),
            task_id: None,
            title: "LiveSsoCallback".into(),
            prompt_preview: None,
            agent_type: Some("explore".into()),
            model: None,
            metadata_json: None,
        })
        .await
        .expect("session");
    let mut opts = anycode_dashboard::server::TestAppOptions::default();
    opts.harness_graph = true;
    opts.tasks_root = Some(tasks);
    opts.seeded_runtime = Some(runtime);
    opts.accounts_sso = Some(client.clone());
    opts.sso_redirect = Some(redirect.to_string());
    let app = anycode_dashboard::server::app_for_test_custom(&db_path, opts)
        .await
        .expect("callback BFF");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18782")
        .await
        .expect("registered pairing callback port 18782 must be free");
    tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    let origin = "http://127.0.0.1:18782";
    let begin = anycode_harness_cloud818::desktop_pairing::post_pairing(
        origin,
        "/api/harness/pairing/sso/begin",
        None,
        serde_json::json!({}),
    )
    .await
    .expect("sso begin");
    let authorize_url = begin["authorize_url"].as_str().expect("authorize_url");
    assert!(
        authorize_url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A18782%2Fapi%2Fharness%2Fpairing%2Fsso%2Fcallback")
            || authorize_url.contains(redirect),
        "begin must use the registered BFF callback"
    );
    let poll_token = begin["poll_token"].as_str().expect("poll_token");
    follow_authorize_to_bff_callback(&issuer, authorize_url).await;
    let hop = anycode_harness_cloud818::desktop_pairing::post_pairing(
        origin,
        "/api/harness/pairing/sso/poll",
        None,
        serde_json::json!({ "poll_token": poll_token }),
    )
    .await
    .expect("poll after GET callback");
    let token = hop["access_token"]
        .as_str()
        .expect("callback must deposit the SSO token")
        .to_string();
    let identity = client.introspect(&token, None).await.expect("introspect");
    assert!(identity.active);
    db.grant_harness_accounts_member(&project.id, &identity.sub.to_string())
        .await
        .expect("grant");
    let challenge = anycode_harness_cloud818::desktop_pairing::post_pairing(
        origin,
        "/api/harness/pairing/challenges",
        Some(&token),
        serde_json::json!({"label":"sso-callback-desktop"}),
    )
    .await
    .expect("challenge");
    let confirm = anycode_harness_cloud818::desktop_pairing::post_pairing(
        origin,
        "/api/harness/pairing/confirm",
        Some(&token),
        serde_json::json!({ "challenge": challenge["challenge"] }),
    )
    .await
    .expect("confirm");
    let device_id = confirm["device_id"].as_str().expect("device_id");
    let device_token = confirm["token"].as_str().expect("token");
    let helper = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/anycode-desktop/resources/bin/anycode-apple-media");
    anycode_apple_media::harness_device_token_set(&[helper.clone()], device_id, device_token)
        .expect("keychain");
    let stored = anycode_apple_media::harness_device_token_get(&[helper], device_id).expect("read");
    assert_eq!(stored.as_deref(), Some(device_token));
    let _ = std::process::Command::new("security")
        .args([
            "delete-generic-password",
            "-s",
            "anycode.harness.device",
            "-a",
            device_id,
        ])
        .status();
    let started = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(90))
        .build()
        .unwrap()
        .post(format!(
            "{origin}/api/projects/{}/harness/graphs/start",
            project.id
        ))
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .header("x-anycode-device-token", device_token)
        .json(&serde_json::json!({
            "graph": {
                "version": 1,
                "name": "live-sso-callback-fileread",
                "nodes": [{
                    "id": "read",
                    "kind": {
                        "type": "work",
                        "agent": "explore",
                        "prompt": FILE_READ_PROMPT
                    }
                }]
            },
            "session_id": session.id
        }))
        .send()
        .await
        .expect("graph start");
    let st = started.status();
    let body: serde_json::Value = started.json().await.unwrap_or(serde_json::json!({}));
    assert!(st.is_success(), "{st} {body}");
    assert!(
        body.to_string().contains(MARKER),
        "BFF callback landing + pairing + GraphRunner must return the file marker"
    );
}

async fn wait_for_marker_in_session(db: &anycode_dashboard::DashboardDb, session_id: &str) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(90);
    loop {
        let records = db
            .list_chat_turn_events(session_id, None, 10_000)
            .await
            .expect("chat events");
        let blob = serde_json::to_string(&records).expect("events json");
        if blob.contains(MARKER) {
            return;
        }
        if tokio::time::Instant::now() > deadline {
            let kinds: Vec<&str> = records.iter().map(|r| r.kind.as_str()).collect();
            panic!(
                "Workbench ChatRuntimeHost did not persist the file marker; events={} kinds={kinds:?}",
                records.len()
            );
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn graph_json_req(
    app: axum::Router,
    method: &str,
    uri: &str,
    body: serde_json::Value,
    bearer: Option<&str>,
    device_token: Option<&str>,
) -> (axum::http::StatusCode, serde_json::Value) {
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    let mut builder = axum::http::Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json");
    if let Some(token) = bearer {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if let Some(token) = device_token {
        builder = builder.header("x-anycode-device-token", token);
    }
    let res = app
        .oneshot(
            builder
                .body(axum::body::Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&bytes).unwrap_or(serde_json::json!({}));
    (status, json)
}

async fn follow_authorize_to_bff_callback(issuer: &str, authorize_url: &str) {
    let origin = issuer.trim_end_matches('/');
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(4))
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let login = http
        .post(format!("{origin}/api/v1/auth/otp/login"))
        .json(&serde_json::json!({"phone":"13800138000","code":"000000"}))
        .send()
        .await
        .expect("otp login");
    assert!(login.status().is_success(), "otp login {}", login.status());
    let cookie = login
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|c| {
            c.split(';')
                .next()
                .filter(|part| part.starts_with("lx_account="))
                .map(str::to_string)
        })
        .expect("lx_account cookie");
    let landed = http
        .get(authorize_url)
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .expect("follow authorize");
    assert!(
        landed.status().is_success(),
        "callback landing {}",
        landed.status()
    );
    assert_eq!(
        landed.url().path(),
        "/api/harness/pairing/sso/callback",
        "issuer must redirect to the pairing BFF callback, not hop/v2"
    );
    let page = landed.text().await.expect("callback html");
    assert!(
        page.contains("access token is not in this page"),
        "callback page must not echo the SSO token"
    );
}

async fn capture_sso_authorize_code(issuer: &str, authorize_url: &str) -> (String, String) {
    let origin = issuer.trim_end_matches('/');
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let login = http
        .post(format!("{origin}/api/v1/auth/otp/login"))
        .json(&serde_json::json!({"phone":"13800138000","code":"000000"}))
        .send()
        .await
        .expect("otp login");
    assert!(login.status().is_success(), "otp login {}", login.status());
    let cookie = login
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|c| {
            c.split(';')
                .next()
                .filter(|part| part.starts_with("lx_account="))
                .map(str::to_string)
        })
        .expect("lx_account cookie");
    let authorize = http
        .get(authorize_url)
        .header(reqwest::header::COOKIE, &cookie)
        .send()
        .await
        .expect("authorize hop");
    assert_eq!(authorize.status(), reqwest::StatusCode::SEE_OTHER);
    let loc = authorize
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|v| v.to_str().ok())
        .expect("redirect");
    let url = reqwest::Url::parse(loc).expect("callback");
    let code = url
        .query_pairs()
        .find(|(k, _)| k == "code")
        .map(|(_, v)| v.into_owned())
        .expect("code");
    let state = url
        .query_pairs()
        .find(|(k, _)| k == "state")
        .map(|(_, v)| v.into_owned())
        .expect("state");
    (code, state)
}

async fn mint_sso_access_token(issuer: &str, secret: &str, redirect: &str) -> String {
    use base64::Engine;
    let origin = issuer.trim_end_matches('/');
    let http = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap();
    let login = http
        .post(format!("{origin}/api/v1/auth/otp/login"))
        .json(&serde_json::json!({"phone":"13800138000","code":"000000"}))
        .send()
        .await
        .expect("otp login");
    assert!(login.status().is_success(), "otp login {}", login.status());
    let cookie = login
        .headers()
        .get_all(reqwest::header::SET_COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|c| {
            c.split(';')
                .next()
                .filter(|part| part.starts_with("lx_account="))
                .map(str::to_string)
        })
        .expect("lx_account cookie");
    let verifier = format!("Vv-_{}", "a".repeat(39));
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .encode(sha2::Sha256::digest(verifier.as_bytes()));
    let authorize = http
        .get(format!("{origin}/api/v2/sso/authorize"))
        .query(&[
            ("client_id", "anycode"),
            ("redirect_uri", redirect),
            ("state", "harness-combined"),
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
        .json(&serde_json::json!({
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

async fn host_execute_named(
    runtime: &anycode_agent::AgentRuntime,
    working_directory: &str,
    id: &str,
    name: &str,
    input: serde_json::Value,
) -> Result<anycode_core::ToolOutput, anycode_core::CoreError> {
    runtime
        .host_execute_tool_call(
            Uuid::new_v4(),
            &AgentType::new("explore"),
            working_directory,
            &anycode_core::ToolCall {
                id: id.into(),
                name: name.into(),
                input,
            },
        )
        .await
}
