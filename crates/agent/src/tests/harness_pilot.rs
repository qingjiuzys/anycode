//! Real Host Pilot: Kernel.run + FileRead through the existing security pipeline.
//! Scripted streams still go through AnyCodeHost; they are not a fake Tool.execute path.

use super::support::{DummyMemoryStore, MockLLM};
use crate::{
    harness_skill_allowlist, AgentClaudeToolGating, AgentRuntime, KernelChildExecutor,
    ReadOnlyPilotBoundary, RuntimeCoreDeps, RuntimeHostFactory, RuntimeMemoryOptions,
    RuntimePromptConfig, RuntimeToolPolicy,
};
use anycode_core::prelude::*;
use anycode_core::{
    harness_readonly_pilot_enabled, ANYCODE_REASONING_CONTENT_METADATA_KEY,
    ANYCODE_TOOL_CALLS_METADATA_KEY,
};
use anycode_harness_core::{
    budget::BudgetPool,
    events::PreviewBus,
    journal::MemoryJournal,
    kernel::{ControlQueue, Host, Kernel},
    types::Limits,
    Capabilities, Error, RunContext, Scope,
};
use anycode_harness_extensions::subagents::{Assignment, Supervisor};
use anycode_security::SecurityLayer;
use anycode_tools::FileReadTool;
use serde_json::json;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use uuid::Uuid;

const MARKER: &str = "HARNESS_PILOT_UNIQUE_MARKER_9f3c";

fn model() -> ModelConfig {
    ModelConfig {
        provider: LLMProvider::Custom("mock".into()),
        model: "mock-native".into(),
        ..Default::default()
    }
}

fn make_runtime(llm: Arc<MockLLM>) -> Arc<AgentRuntime> {
    let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
    tools.insert("FileRead".into(), Box::new(FileReadTool::new(true)));
    Arc::new(AgentRuntime::new(
        RuntimeCoreDeps {
            llm_client: llm,
            tools,
            memory_store: Arc::new(DummyMemoryStore),
            default_model_config: model(),
            model_overrides: HashMap::new(),
            failover_chain: vec![],
            disk_output: None,
            security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
            sandbox_mode: true,
            prompt_config: RuntimePromptConfig::default(),
        },
        RuntimeMemoryOptions {
            memory_pipeline: None,
            memory_pipeline_settings: None,
            memory_project_autosave_enabled: false,
            session_notifications: None,
            automem: None,
            automem_base_path: None,
        },
        RuntimeToolPolicy {
            tool_name_deny: vec![],
            claude_gating: AgentClaudeToolGating::default(),
            expose_skill_on_explore_plan: false,
        },
    ))
}

fn root_ctx() -> RunContext {
    RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new(["fs.read".into()]).unwrap(),
        BudgetPool::new(10_000).unwrap(),
        Duration::from_secs(8),
    )
    .unwrap()
}

fn file_read_call(path: &str) -> ToolCall {
    ToolCall {
        id: "call-read-1".into(),
        name: "FileRead".into(),
        input: json!({"file_path": path}),
    }
}

fn workspace_with_note() -> (TempDir, String) {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("note.txt"), format!("hello {MARKER}\n")).unwrap();
    (dir, "note.txt".into())
}

fn limits() -> Limits {
    Limits {
        reservation_per_hop: 64,
        ..Limits::default()
    }
}

#[tokio::test]
async fn readonly_pilot_reads_real_file_through_security_pipeline() {
    let (dir, rel) = workspace_with_note();
    let call = file_read_call(&rel);
    let llm = Arc::new(MockLLM::with_stream_batches(
        vec![],
        vec![
            vec![
                StreamEvent::Reasoning("provider-private chain".into()),
                StreamEvent::ToolCall(call.clone()),
                StreamEvent::Usage(Usage {
                    input_tokens: 11,
                    output_tokens: 7,
                    cache_creation_tokens: None,
                    cache_read_tokens: None,
                }),
                StreamEvent::Done,
            ],
            vec![
                StreamEvent::Delta("the note contains the marker".into()),
                StreamEvent::Usage(Usage {
                    input_tokens: 5,
                    output_tokens: 3,
                    cache_creation_tokens: None,
                    cache_read_tokens: None,
                }),
                StreamEvent::Done,
            ],
        ],
    ));
    let runtime = make_runtime(llm);
    let ctx = root_ctx();
    let journal = MemoryJournal::default();
    let controls = ControlQueue::default();
    let history = runtime
        .run_readonly_pilot(
            &ctx,
            dir.path(),
            "read note.txt",
            &journal,
            &controls,
            limits(),
        )
        .await
        .expect("pilot should complete a real FileRead");

    let records = journal.records().unwrap();
    let intent = records
        .iter()
        .position(|r| r.event.kind == "tool_intent")
        .expect("tool_intent before execute");
    let end = records
        .iter()
        .position(|r| r.event.kind == "tool_end")
        .expect("tool_end after execute");
    assert!(intent < end);
    let result_text = records[end].event.data["result"]["value"].to_string();
    assert!(
        result_text.contains(MARKER),
        "FileRead must return the real file bytes, got {result_text}"
    );

    let usage: Vec<_> = records.iter().filter(|r| r.event.kind == "usage").collect();
    assert_eq!(usage.len(), 2);
    assert!(usage.iter().all(|r| r.event.data["measured"] == true));
    let spent = ctx.budget().snapshot().unwrap().spent;
    assert_eq!(spent, 11 + 7 + 5 + 3);

    let first: Message = serde_json::from_value(history[1].clone()).unwrap();
    assert_eq!(
        first.metadata.get(ANYCODE_REASONING_CONTENT_METADATA_KEY),
        Some(&json!("provider-private chain"))
    );
    assert_eq!(
        first.metadata[ANYCODE_TOOL_CALLS_METADATA_KEY][0]["id"],
        "call-read-1"
    );
    let previews_had_reasoning = records.iter().any(|r| {
        r.event.data.to_string().contains("provider-private chain")
            && r.event.kind != "message_committed"
    });
    assert!(
        !previews_had_reasoning,
        "provider-private reasoning must not be copied into UI preview events"
    );
}

#[tokio::test]
async fn done_without_calls_does_not_execute_tools() {
    let (dir, _) = workspace_with_note();
    let llm = Arc::new(MockLLM::with_stream_batches(
        vec![],
        vec![vec![
            StreamEvent::Delta("no tools needed".into()),
            StreamEvent::Usage(Usage {
                input_tokens: 2,
                output_tokens: 2,
                cache_creation_tokens: None,
                cache_read_tokens: None,
            }),
            StreamEvent::Done,
        ]],
    ));
    let runtime = make_runtime(llm);
    let ctx = root_ctx();
    let journal = MemoryJournal::default();
    let controls = ControlQueue::default();
    runtime
        .run_readonly_pilot(
            &ctx,
            dir.path(),
            "just answer",
            &journal,
            &controls,
            limits(),
        )
        .await
        .unwrap();
    let records = journal.records().unwrap();
    assert!(!records.iter().any(|r| r.event.kind == "tool_intent"));
    assert!(!records.iter().any(|r| r.event.kind == "tool_end"));
}

#[tokio::test]
async fn stream_without_done_discards_partial_and_does_not_run_tools() {
    let (dir, rel) = workspace_with_note();
    let llm = Arc::new(MockLLM::with_stream_batches(
        vec![],
        vec![vec![
            StreamEvent::Delta("half".into()),
            StreamEvent::ToolCall(file_read_call(&rel)),
        ]],
    ));
    let runtime = make_runtime(llm);
    let ctx = root_ctx();
    let journal = MemoryJournal::default();
    let controls = ControlQueue::default();
    let err = runtime
        .run_readonly_pilot(
            &ctx,
            dir.path(),
            "read note.txt",
            &journal,
            &controls,
            limits(),
        )
        .await
        .expect_err("incomplete stream must fail closed");
    assert!(matches!(err, Error::Host(_)), "{err:?}");
    assert!(!journal
        .records()
        .unwrap()
        .iter()
        .any(|r| r.event.kind == "tool_intent"));
}

#[tokio::test]
async fn parent_cancel_stops_pilot_before_tools() {
    let (dir, _) = workspace_with_note();
    let llm = Arc::new(MockLLM::with_stream_batches(vec![], vec![]));
    let runtime = make_runtime(llm);
    let ctx = root_ctx();
    ctx.cancel();
    let journal = MemoryJournal::default();
    let controls = ControlQueue::default();
    let err = runtime
        .run_readonly_pilot(
            &ctx,
            dir.path(),
            "read note.txt",
            &journal,
            &controls,
            limits(),
        )
        .await
        .expect_err("cancelled context must not start tools");
    assert!(matches!(err, Error::Cancelled), "{err:?}");
    assert!(!journal
        .records()
        .unwrap()
        .iter()
        .any(|r| r.event.kind == "tool_intent"));
}

#[tokio::test]
async fn enterprise_scope_is_rejected_by_pilot_boundary() {
    let ctx = RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: Some(Uuid::new_v4()),
            tenant: Some(Uuid::new_v4()),
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new(["fs.read".into()]).unwrap(),
        BudgetPool::new(100).unwrap(),
        Duration::from_secs(2),
    )
    .unwrap();
    let dir = TempDir::new().unwrap();
    let err = match ReadOnlyPilotBoundary::new(&ctx, dir.path()) {
        Ok(_) => panic!("enterprise needs ProductAcl"),
        Err(err) => err,
    };
    assert!(matches!(err, Error::Denied(_)), "{err:?}");
}

#[tokio::test]
async fn harness_host_plus_kernel_is_the_only_invoke_path() {
    let (dir, rel) = workspace_with_note();
    let llm = Arc::new(MockLLM::with_stream_batches(
        vec![],
        vec![
            vec![
                StreamEvent::ToolCall(file_read_call(&rel)),
                StreamEvent::Usage(Usage {
                    input_tokens: 4,
                    output_tokens: 1,
                    cache_creation_tokens: None,
                    cache_read_tokens: None,
                }),
                StreamEvent::Done,
            ],
            vec![StreamEvent::Done],
        ],
    ));
    let runtime = make_runtime(llm);
    let ctx = root_ctx();
    let mut bindings = BTreeMap::new();
    bindings.insert("FileRead".into(), ("fs.read".into(), true));
    let host = runtime
        .harness_host(
            &ctx,
            dir.path(),
            AgentType::new("explore"),
            model(),
            bindings,
            Arc::new(ReadOnlyPilotBoundary::new(&ctx, dir.path()).unwrap()),
            PreviewBus::default(),
            std::sync::Arc::new(crate::RunLifecycle::default()),
            crate::HarnessHostPolicy::default(),
        )
        .await
        .unwrap();
    let journal = MemoryJournal::default();
    let controls = ControlQueue::default();
    let kernel = Kernel {
        host: &host,
        journal: &journal,
        previews: PreviewBus::default(),
        controls: &controls,
        limits: limits(),
    };
    kernel
        .run(&ctx, vec![host.user_message("read the note").unwrap()])
        .await
        .unwrap();
    assert!(journal
        .records()
        .unwrap()
        .iter()
        .any(|r| r.event.kind == "tool_end"));
}

#[tokio::test]
async fn harness_host_rejects_agent_binding_on_pilot() {
    let runtime = make_runtime(Arc::new(MockLLM::new(vec![])));
    let ctx = root_ctx();
    let dir = TempDir::new().unwrap();
    let mut bindings = BTreeMap::new();
    bindings.insert("FileRead".into(), ("fs.read".into(), true));
    bindings.insert("Agent".into(), ("agent.spawn".into(), false));
    let err = match runtime
        .harness_host(
            &ctx,
            dir.path(),
            AgentType::new("explore"),
            model(),
            bindings,
            Arc::new(ReadOnlyPilotBoundary::new(&ctx, dir.path()).unwrap()),
            PreviewBus::default(),
            std::sync::Arc::new(crate::RunLifecycle::default()),
            crate::HarnessHostPolicy::default(),
        )
        .await
    {
        Ok(_) => panic!("pilot must keep rejecting Agent bindings"),
        Err(err) => err,
    };
    assert!(matches!(err, Error::Denied(_)), "{err:?}");
}

#[test]
fn experiment_switch_defaults_off() {
    if std::env::var_os("ANYCODE_HARNESS_V1_READONLY_PILOT").is_none() {
        assert!(!harness_readonly_pilot_enabled(&FeatureRegistry::default()));
    }
}

#[tokio::test]
async fn supervisor_child_uses_same_kernel_and_shared_budget() {
    let (dir, rel) = workspace_with_note();
    let llm = Arc::new(MockLLM::with_stream_batches(
        vec![],
        vec![
            vec![
                StreamEvent::ToolCall(file_read_call(&rel)),
                StreamEvent::Usage(Usage {
                    input_tokens: 6,
                    output_tokens: 2,
                    cache_creation_tokens: None,
                    cache_read_tokens: None,
                }),
                StreamEvent::Done,
            ],
            vec![
                StreamEvent::Delta(format!("child saw {MARKER}")),
                StreamEvent::Usage(Usage {
                    input_tokens: 3,
                    output_tokens: 1,
                    cache_creation_tokens: None,
                    cache_read_tokens: None,
                }),
                StreamEvent::Done,
            ],
        ],
    ));
    let runtime = make_runtime(llm);
    let parent = RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new(["fs.read".into(), "agent.spawn".into()]).unwrap(),
        BudgetPool::new(10_000).unwrap(),
        Duration::from_secs(8),
    )
    .unwrap();
    let journal = Arc::new(MemoryJournal::default());
    let executor =
        KernelChildExecutor::new(runtime, dir.path().to_path_buf(), journal.clone(), limits());
    let supervisor = Supervisor::new(&parent, 2, 4).unwrap();
    let outcome = supervisor
        .run(
            &parent,
            Assignment {
                agent: "explore".into(),
                prompt: "read note.txt".into(),
                capabilities: Capabilities::new(["fs.read".into()]).unwrap(),
            },
            &executor,
        )
        .await
        .unwrap();
    assert_eq!(parent.depth(), 0);
    assert_eq!(parent.budget().snapshot().unwrap().spent, 6 + 2 + 3 + 1);
    assert!(
        outcome.output["text"].as_str().unwrap().contains(MARKER),
        "child must return the real FileRead marker"
    );
    assert!(!outcome.partial);
}

#[tokio::test]
async fn write_child_is_rejected_without_worktree() {
    let dir = TempDir::new().unwrap();
    let runtime = make_runtime(Arc::new(MockLLM::with_stream_batches(vec![], vec![])));
    let parent = RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new(["fs.read".into(), "fs.write".into(), "agent.spawn".into()]).unwrap(),
        BudgetPool::new(1000).unwrap(),
        Duration::from_secs(4),
    )
    .unwrap();
    let executor = KernelChildExecutor::new(
        runtime,
        dir.path().to_path_buf(),
        Arc::new(MemoryJournal::default()),
        limits(),
    );
    let supervisor = Supervisor::new(&parent, 1, 4).unwrap();
    let err = supervisor
        .run(
            &parent,
            Assignment {
                agent: "implementer".into(),
                prompt: "edit files".into(),
                capabilities: Capabilities::new(["fs.write".into()]).unwrap(),
            },
            &executor,
        )
        .await
        .expect_err("write child must not share the parent workspace");
    assert!(matches!(err, Error::Denied(_)), "{err:?}");
}

#[tokio::test]
async fn graph_verifier_ids_are_not_implemented() {
    let dir = TempDir::new().unwrap();
    let runtime = make_runtime(Arc::new(MockLLM::with_stream_batches(vec![], vec![])));
    let ctx = root_ctx();
    let factory = RuntimeHostFactory::new(runtime, dir.path().to_path_buf(), PreviewBus::default());
    let err = anycode_harness_host::graph_adapter::AgentHostFactory::verify(
        &factory,
        &ctx,
        "tests-pass",
        BTreeMap::new(),
    )
    .await
    .expect_err("sample verifier IDs must fail closed");
    assert!(matches!(err, Error::Unsupported(_)), "{err:?}");
}

#[tokio::test]
async fn readonly_graph_host_may_run_without_sandbox_mode() {
    let (dir, _) = workspace_with_note();
    let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
    tools.insert("FileRead".into(), Box::new(FileReadTool::new(true)));
    let runtime = Arc::new(AgentRuntime::new(
        RuntimeCoreDeps {
            llm_client: Arc::new(MockLLM::new(vec![])),
            tools,
            memory_store: Arc::new(DummyMemoryStore),
            default_model_config: model(),
            model_overrides: HashMap::new(),
            failover_chain: vec![],
            disk_output: None,
            security: Arc::new(SecurityLayer::new(PermissionMode::BypassPermissions)),
            sandbox_mode: false,
            prompt_config: RuntimePromptConfig::default(),
        },
        RuntimeMemoryOptions {
            memory_pipeline: None,
            memory_pipeline_settings: None,
            memory_project_autosave_enabled: false,
            session_notifications: None,
            automem: None,
            automem_base_path: None,
        },
        RuntimeToolPolicy {
            tool_name_deny: vec![],
            claude_gating: AgentClaudeToolGating::default(),
            expose_skill_on_explore_plan: false,
        },
    ));
    let ctx = root_ctx();
    let mut readonly = BTreeMap::new();
    readonly.insert("FileRead".into(), ("fs.read".into(), true));
    let boundary = Arc::new(ReadOnlyPilotBoundary::new(&ctx, dir.path()).unwrap());
    let lifecycle = std::sync::Arc::new(crate::RunLifecycle::default());
    assert!(
        runtime
            .harness_host(
                &ctx,
                dir.path(),
                AgentType::new("explore"),
                model(),
                readonly.clone(),
                boundary.clone(),
                PreviewBus::default(),
                lifecycle.clone(),
                crate::HarnessHostPolicy::default(),
            )
            .await
            .is_err(),
        "pilot default still requires sandbox_mode"
    );
    assert!(runtime
        .harness_host(
            &ctx,
            dir.path(),
            AgentType::new("explore"),
            model(),
            readonly,
            boundary.clone(),
            PreviewBus::default(),
            lifecycle.clone(),
            crate::HarnessHostPolicy::readonly_graph(),
        )
        .await
        .is_ok());
    let mut mutating = BTreeMap::new();
    mutating.insert("FileRead".into(), ("fs.read".into(), false));
    assert!(
        runtime
            .harness_host(
                &ctx,
                dir.path(),
                AgentType::new("explore"),
                model(),
                mutating,
                boundary,
                PreviewBus::default(),
                lifecycle,
                crate::HarnessHostPolicy::readonly_graph(),
            )
            .await
            .is_err(),
        "unsandboxed graph host is still read-only"
    );
}

#[test]
fn skill_markdown_cannot_grant_capabilities() {
    let gov = anycode_tools::SkillsGovernance {
        global_allowlist: Some(vec!["harness-code-archaeology".into()]),
        agent_allowlists: Default::default(),
        project_enabled: Some(["harness-code-archaeology".into()].into_iter().collect()),
    };
    let allow = harness_skill_allowlist(&gov, "explore");
    assert_eq!(
        allow.into_iter().collect::<Vec<_>>(),
        vec!["harness-code-archaeology".to_string()]
    );
}
