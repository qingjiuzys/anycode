//! GraphEngine::run through AgentRuntime::execute_task (multi-agent DAG).
//!
//! These tests use a scripted LLM — not a live provider — but they do exercise
//! the real orchestration path: topo layers → per-step agent lookup → execute_task
//! → artifact handoff into the next prompt.

use super::support::{msg_text, DummyMemoryStore, EchoTool};
use crate::{
    AgentClaudeToolGating, AgentRuntime, GraphEngine, GraphRunOptions, RuntimeCoreDeps,
    RuntimeMemoryOptions, RuntimePromptConfig, RuntimeToolPolicy,
};
use anycode_core::prelude::*;
use anycode_core::{WorkflowDefinition, WorkflowRetry, WorkflowStep};
use anycode_security::SecurityLayer;
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use uuid::Uuid;

struct RecordedCall {
    system: String,
    user: String,
}

struct SequenceLlm {
    outcomes: Mutex<VecDeque<Result<LLMResponse, String>>>,
    calls: Mutex<Vec<RecordedCall>>,
}

impl SequenceLlm {
    fn new(outcomes: Vec<Result<LLMResponse, String>>) -> Arc<Self> {
        Arc::new(Self {
            outcomes: Mutex::new(outcomes.into()),
            calls: Mutex::new(Vec::new()),
        })
    }

    fn recorded(&self) -> Vec<RecordedCall> {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| RecordedCall {
                system: c.system.clone(),
                user: c.user.clone(),
            })
            .collect()
    }

    fn call_count(&self) -> usize {
        self.calls.lock().unwrap().len()
    }
}

#[async_trait]
impl LLMClient for SequenceLlm {
    async fn chat(
        &self,
        messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        let system = messages
            .iter()
            .find(|m| m.role == MessageRole::System)
            .map(message_text)
            .unwrap_or_default();
        let user = messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .map(message_text)
            .collect::<Vec<_>>()
            .join("\n");
        self.calls
            .lock()
            .unwrap()
            .push(RecordedCall { system, user });
        match self.outcomes.lock().unwrap().pop_front() {
            Some(Ok(resp)) => Ok(resp),
            Some(Err(err)) => Err(CoreError::LLMError(err)),
            None => Err(CoreError::LLMError("mock queue empty".into())),
        }
    }

    async fn chat_stream(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: &ModelConfig,
    ) -> Result<tokio::sync::mpsc::Receiver<StreamEvent>, CoreError> {
        Err(CoreError::LLMError(
            "graph tests use chat(), not stream".into(),
        ))
    }
}

fn message_text(message: &Message) -> String {
    match &message.content {
        MessageContent::Text(t) => t.clone(),
        other => format!("{other:?}"),
    }
}

fn text_response(text: &str) -> LLMResponse {
    LLMResponse {
        message: msg_text(MessageRole::Assistant, text),
        tool_calls: vec![],
        usage: Usage {
            input_tokens: 1,
            output_tokens: 1,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        },
    }
}

fn ok_text(text: &str) -> Result<LLMResponse, String> {
    Ok(text_response(text))
}

fn make_runtime(llm: Arc<SequenceLlm>, disk: DiskTaskOutput) -> AgentRuntime {
    let mut tools: HashMap<ToolName, Box<dyn Tool>> = HashMap::new();
    tools.insert("Echo".to_string(), Box::new(EchoTool));
    AgentRuntime::new(
        RuntimeCoreDeps {
            llm_client: llm,
            tools,
            memory_store: Arc::new(DummyMemoryStore),
            default_model_config: ModelConfig {
                provider: LLMProvider::Custom("mock".to_string()),
                model: "mock".to_string(),
                base_url: None,
                temperature: None,
                max_tokens: None,
                api_key: None,
                ..Default::default()
            },
            model_overrides: HashMap::new(),
            failover_chain: vec![],
            disk_output: Some(disk),
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
    )
}

fn run_opts(workspace: &TempDir, checkpoint: Option<std::path::PathBuf>) -> GraphRunOptions {
    GraphRunOptions {
        working_directory: workspace.path().to_path_buf(),
        user_prompt: Some("user asked for a multi-agent pass".into()),
        dashboard_session_id: Some("sess-graph-test".into()),
        nested_cancel: None,
        checkpoint_path: checkpoint,
    }
}

fn step_status<'a>(result: &'a crate::GraphRunResult, id: &str) -> &'a str {
    result
        .steps
        .iter()
        .find(|s| s.step_id == id)
        .map(|s| s.status.as_str())
        .unwrap_or("<missing>")
}

fn assert_agent_in_log(disk: &DiskTaskOutput, task_id: &str, agent: &str) {
    let tid = Uuid::parse_str(task_id).expect("graph step task_id");
    let log = disk.tail(tid, 64 * 1024).expect("task log");
    assert!(
        log.contains(&format!("[task_start] agent_type={agent}")),
        "expected agent_type={agent} in log, got:\n{log}"
    );
}

fn wf_step(
    id: &str,
    agent: &str,
    prompt: &str,
    depends_on: &[&str],
    when: Option<&str>,
) -> WorkflowStep {
    WorkflowStep {
        id: id.into(),
        prompt: prompt.into(),
        agent: Some(agent.into()),
        depends_on: depends_on.iter().map(|s| (*s).to_string()).collect(),
        when: when.map(|s| s.to_string()),
        ..Default::default()
    }
}

#[tokio::test]
async fn sample_three_node_runs_explore_then_plan_then_general_purpose() {
    let workspace = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let disk = DiskTaskOutput::new(output.path().to_path_buf());
    let llm = SequenceLlm::new(vec![
        ok_text("SCOUT_ARTIFACT"),
        ok_text("PLAN_ARTIFACT"),
        ok_text("SUMMARY_ARTIFACT"),
    ]);
    let runtime = make_runtime(llm.clone(), disk.clone());
    let wf = GraphEngine::sample_three_node_workflow();

    let result = GraphEngine::run(&runtime, &wf, run_opts(&workspace, None))
        .await
        .unwrap();

    assert_eq!(result.status, "completed");
    assert_eq!(
        result.layers,
        vec![
            vec!["scout".to_string()],
            vec!["plan".to_string()],
            vec!["summarize".to_string()],
        ]
    );
    assert_eq!(result.steps.len(), 3);
    assert_eq!(step_status(&result, "scout"), "passed");
    assert_eq!(step_status(&result, "plan"), "passed");
    assert_eq!(step_status(&result, "summarize"), "passed");
    assert_eq!(result.steps[0].summary, "SCOUT_ARTIFACT");
    assert_eq!(result.steps[1].summary, "PLAN_ARTIFACT");
    assert_eq!(result.steps[2].summary, "SUMMARY_ARTIFACT");

    assert_agent_in_log(
        &disk,
        result.steps[0].task_id.as_deref().unwrap(),
        "explore",
    );
    assert_agent_in_log(&disk, result.steps[1].task_id.as_deref().unwrap(), "plan");
    assert_agent_in_log(
        &disk,
        result.steps[2].task_id.as_deref().unwrap(),
        "general-purpose",
    );

    let calls = llm.recorded();
    assert_eq!(calls.len(), 3);
    assert!(
        calls[0].system.contains("exploring codebases"),
        "scout should bind ExploreAgent"
    );
    assert!(
        calls[1].system.contains("designing implementation plans"),
        "plan should bind PlanAgent"
    );
    assert!(
        calls[2].system.contains("researching complex questions"),
        "summarize should bind GeneralPurposeAgent"
    );
    assert!(calls[0].user.contains("step_id: scout"));
    assert!(calls[1].user.contains("## Artifact from scout"));
    assert!(calls[1].user.contains("SCOUT_ARTIFACT"));
    assert!(calls[2].user.contains("## Artifact from plan"));
    assert!(calls[2].user.contains("PLAN_ARTIFACT"));
}

#[tokio::test]
async fn diamond_join_runs_both_branch_agents_then_handoff() {
    let workspace = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let disk = DiskTaskOutput::new(output.path().to_path_buf());
    let llm = SequenceLlm::new(vec![
        ok_text("LEFT_ARTIFACT"),
        ok_text("RIGHT_ARTIFACT"),
        ok_text("JOIN_ARTIFACT"),
    ]);
    let runtime = make_runtime(llm.clone(), disk);
    let wf = WorkflowDefinition {
        name: "diamond".into(),
        mode: Some("code".into()),
        steps: vec![
            wf_step("left", "explore", "scout left", &[], None),
            wf_step("right", "plan", "scout right", &[], None),
            wf_step(
                "join",
                "general-purpose",
                "merge both branches",
                &["left", "right"],
                None,
            ),
        ],
        ..Default::default()
    };

    let result = GraphEngine::run(&runtime, &wf, run_opts(&workspace, None))
        .await
        .unwrap();

    assert_eq!(result.status, "completed");
    assert_eq!(
        result.layers,
        vec![
            vec!["left".to_string(), "right".to_string()],
            vec!["join".to_string()],
        ]
    );
    assert_eq!(step_status(&result, "left"), "passed");
    assert_eq!(step_status(&result, "right"), "passed");
    assert_eq!(step_status(&result, "join"), "passed");

    let calls = llm.recorded();
    assert_eq!(calls.len(), 3);
    assert!(calls[0].system.contains("exploring codebases"));
    assert!(calls[1].system.contains("designing implementation plans"));
    assert!(calls[2].user.contains("## Artifact from left"));
    assert!(calls[2].user.contains("LEFT_ARTIFACT"));
    assert!(calls[2].user.contains("## Artifact from right"));
    assert!(calls[2].user.contains("RIGHT_ARTIFACT"));
}

#[tokio::test]
async fn checkpoint_rerun_is_refused_without_explicit_harness_resume() {
    let workspace = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let disk = DiskTaskOutput::new(output.path().to_path_buf());
    let ckpt = workspace.path().join("graph-ckpt.json");
    let llm = SequenceLlm::new(vec![
        ok_text("SCOUT_ARTIFACT"),
        ok_text("PLAN_ARTIFACT"),
        ok_text("SUMMARY_ARTIFACT"),
    ]);
    let runtime = make_runtime(llm.clone(), disk);
    let wf = GraphEngine::sample_three_node_workflow();
    let opts = run_opts(&workspace, Some(ckpt.clone()));

    let first = GraphEngine::run(&runtime, &wf, opts.clone()).await.unwrap();
    assert_eq!(first.status, "completed");
    assert_eq!(llm.call_count(), 3);
    assert!(ckpt.exists(), "first run still writes a checkpoint file");

    let second = GraphEngine::run(&runtime, &wf, opts).await;
    let err = second.expect_err("legacy auto-resume must fail closed");
    assert!(
        err.to_string()
            .contains("legacy checkpoint resume is disabled"),
        "unexpected error: {err}"
    );
    assert_eq!(
        llm.call_count(),
        3,
        "refused resume must not call execute_task again"
    );
}

#[tokio::test]
async fn failed_agent_stops_downstream_steps() {
    let workspace = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let disk = DiskTaskOutput::new(output.path().to_path_buf());
    let llm = SequenceLlm::new(vec![ok_text("SCOUT_ARTIFACT"), Err("plan boom".into())]);
    let runtime = make_runtime(llm.clone(), disk);
    let wf = GraphEngine::sample_three_node_workflow();

    let result = GraphEngine::run(&runtime, &wf, run_opts(&workspace, None))
        .await
        .unwrap();

    assert_eq!(result.status, "failed");
    assert_eq!(step_status(&result, "scout"), "passed");
    assert_eq!(step_status(&result, "plan"), "failed");
    assert!(
        !result.steps.iter().any(|s| s.step_id == "summarize"),
        "general-purpose must not run after plan failed"
    );
    assert_eq!(llm.recorded().len(), 2);
}

#[tokio::test]
async fn when_clause_skips_agent_but_dependents_still_run() {
    let workspace = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let disk = DiskTaskOutput::new(output.path().to_path_buf());
    let llm = SequenceLlm::new(vec![ok_text("SCOUT_ARTIFACT"), ok_text("JOIN_ARTIFACT")]);
    let runtime = make_runtime(llm.clone(), disk);
    let wf = WorkflowDefinition {
        name: "when-skip".into(),
        mode: Some("code".into()),
        steps: vec![
            wf_step("scout", "explore", "inspect", &[], None),
            wf_step(
                "maybe",
                "plan",
                "only if magic",
                &["scout"],
                Some("contains:ZZZ_NEVER"),
            ),
            wf_step(
                "join",
                "general-purpose",
                "continue after skip",
                &["maybe"],
                None,
            ),
        ],
        ..Default::default()
    };

    let result = GraphEngine::run(&runtime, &wf, run_opts(&workspace, None))
        .await
        .unwrap();

    assert_eq!(result.status, "completed");
    assert_eq!(step_status(&result, "scout"), "passed");
    assert_eq!(step_status(&result, "maybe"), "skipped");
    assert_eq!(step_status(&result, "join"), "passed");
    let calls = llm.recorded();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].system.contains("exploring codebases"));
    assert!(calls[1].system.contains("researching complex questions"));
}

#[tokio::test]
async fn implicit_side_effect_retry_is_disabled() {
    let workspace = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let disk = DiskTaskOutput::new(output.path().to_path_buf());
    let llm = SequenceLlm::new(vec![Err("transient".into()), ok_text("RECOVERED")]);
    let runtime = make_runtime(llm.clone(), disk);
    let wf = WorkflowDefinition {
        name: "retry-once".into(),
        mode: Some("code".into()),
        retry: Some(WorkflowRetry {
            max_attempts: 2,
            backoff_ms: 0,
        }),
        steps: vec![wf_step("only", "explore", "do it", &[], None)],
        ..Default::default()
    };

    let result = GraphEngine::run(&runtime, &wf, run_opts(&workspace, None))
        .await
        .unwrap();

    assert_eq!(result.status, "failed");
    assert_eq!(step_status(&result, "only"), "failed");
    assert_eq!(
        llm.recorded().len(),
        1,
        "legacy graph must not retry mutating steps without a trusted verifier"
    );
}

#[tokio::test]
async fn required_gates_refuse_before_any_side_effect() {
    let workspace = TempDir::new().unwrap();
    let output = TempDir::new().unwrap();
    let disk = DiskTaskOutput::new(output.path().to_path_buf());
    let llm = SequenceLlm::new(vec![ok_text("SHOULD_NOT_RUN")]);
    let runtime = make_runtime(llm.clone(), disk);
    let mut step = wf_step("only", "explore", "gated", &[], None);
    step.required_gates = vec!["screenshot".into()];
    let wf = WorkflowDefinition {
        name: "gated".into(),
        mode: Some("code".into()),
        steps: vec![step],
        ..Default::default()
    };

    let err = GraphEngine::run(&runtime, &wf, run_opts(&workspace, None))
        .await
        .expect_err("gated legacy workflows must fail closed");
    assert!(
        err.to_string()
            .contains("legacy graph gates have no trusted verifier"),
        "unexpected error: {err}"
    );
    assert_eq!(
        llm.call_count(),
        0,
        "gated workflows must not execute tools or model calls"
    );
}
