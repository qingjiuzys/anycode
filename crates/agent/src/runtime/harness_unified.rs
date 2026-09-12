//! Unified Kernel adapters for execute_task / execute_turn.
//!
//! When the experiment flag is on, those entries convert input and return output.
//! They do not start the legacy agentic loop and the Kernel does not call them back.

use super::agentic_loop::task_cancelled_failure;
use super::harness_bridge::{HarnessBoundary, LifecycleBoundary, RunLifecycle};
use super::live_trace_emit;
use super::task_summary::last_assistant_plain_text;
use super::AgentRuntime;
use anycode_core::prelude::*;
use anycode_core::{Artifact, ExpectedArtifact, GatePlan, TaskFamily};
use anycode_core::{ANYCODE_TOOL_CALLS_METADATA_KEY, NESTED_TASK_COOPERATIVE_CANCEL_ERROR};
use anycode_harness_core::{
    budget::BudgetPool,
    events::PreviewBus,
    journal::{EventSink, MemoryJournal},
    kernel::{ControlQueue, Host, Kernel},
    types::{
        Hop, Invocation, Limits, ProviderMessage, ToolResult, ToolSpec, Usage as HarnessUsage,
    },
    Capabilities, Error, Result as HarnessResult, RunContext, Scope,
};
use anycode_harness_host::AnyCodeHost;
use async_trait::async_trait;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

pub(crate) const FORBIDDEN_UNIFIED_TOOLS: &[&str] = &[
    "Task",
    "CronCreate",
    "RemoteTrigger",
    "ScheduleWakeup",
    "Config",
    "PowerShell",
    "Repl",
];

/// Trusted capability map. Unknown tools keep the existing security pipeline via
/// `legacy.tool`; orchestration names never become bindings.
pub fn capability_for_legacy_tool(name: &str) -> Option<(String, bool)> {
    if FORBIDDEN_UNIFIED_TOOLS.contains(&name) {
        return None;
    }
    Some(match name {
        "FileRead" | "Glob" | "Grep" => ("fs.read".into(), true),
        "FileWrite" | "Edit" | "NotebookEdit" => ("fs.write".into(), false),
        "Skill" | "SkillSearch" | "SkillAppRead" => ("skill.read".into(), true),
        "Agent" => ("agent.spawn".into(), false),
        "Echo" => ("test.echo".into(), true),
        "WebFetch" | "WebSearch" | "KnowledgeSearch" => ("web.read".into(), true),
        "Bash" => ("process.exec".into(), false),
        "AskUserQuestion" => ("user.ask".into(), true),
        "HarnessAgentSpawn" | "HarnessAgentJoin" | "HarnessAgentCancel" => {
            ("agent.spawn".into(), false)
        }
        "HarnessComputerObserve" => ("computer.observe".into(), true),
        "HarnessComputerAct" => ("computer.input".into(), false),
        "HarnessSkillSearch" | "HarnessSkillActivate" => ("skill.read".into(), true),
        "TodoWrite" | "PlanWrite" => ("workspace.write".into(), false),
        "ToolSearch" => ("tool.search".into(), true),
        "StructuredOutput" => ("output.structured".into(), true),
        "SkillAppPresent" | "SkillAppPush" => ("skill.app".into(), false),
        other if other.starts_with("mcp__") => return None,
        _ => ("legacy.tool".into(), false),
    })
}

fn is_workspace_file_tool(name: &str) -> bool {
    matches!(name, "FileRead" | "FileWrite" | "Edit")
}

fn invocation_file_path(call: &Invocation) -> HarnessResult<&str> {
    call.arguments
        .get("file_path")
        .or_else(|| call.arguments.get("path"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Invalid("file tool path".into()))
}

fn resolve_under_root(root: &Path, path: &str, denied: &str) -> HarnessResult<PathBuf> {
    let raw = Path::new(path);
    let candidate = if raw.is_absolute() {
        raw.to_path_buf()
    } else {
        root.join(raw)
    };
    let resolved = std::fs::canonicalize(&candidate).or_else(|_| {
        candidate
            .parent()
            .and_then(|parent| std::fs::canonicalize(parent).ok())
            .map(|parent| parent.join(candidate.file_name().unwrap_or_default()))
            .ok_or_else(|| Error::Denied(denied.into()))
    })?;
    if !resolved.starts_with(root) {
        return Err(Error::Denied(denied.into()));
    }
    Ok(resolved)
}

/// Local / project scope only. Enterprise tenant requires a live ProductAcl (M6).
pub struct UnifiedTaskBoundary {
    scope: String,
    allowed: BTreeMap<String, (String, bool)>,
    root: PathBuf,
}

impl UnifiedTaskBoundary {
    pub fn new(
        ctx: &RunContext,
        root: &Path,
        allowed: BTreeMap<String, (String, bool)>,
    ) -> HarnessResult<Self> {
        if ctx.scope().tenant.is_some() {
            return Err(Error::Denied(
                "enterprise requires live ProductAcl boundary".into(),
            ));
        }
        if allowed.is_empty() {
            return Err(Error::Invalid("unified host has no tool bindings".into()));
        }
        Ok(Self {
            scope: ctx.scope().binding()?,
            allowed,
            root: std::fs::canonicalize(root)?,
        })
    }
}

#[async_trait]
impl HarnessBoundary for UnifiedTaskBoundary {
    async fn preflight(&self, ctx: &RunContext, call: &Invocation) -> HarnessResult<()> {
        ctx.check()?;
        if ctx.scope().binding()? != self.scope {
            return Err(Error::Denied("unified scope mismatch".into()));
        }
        if !self.allowed.contains_key(&call.name) {
            return Err(Error::Denied("tool outside unified allowlist".into()));
        }
        if is_workspace_file_tool(&call.name) {
            resolve_under_root(
                &self.root,
                invocation_file_path(call)?,
                "file path outside unified root",
            )?;
        }
        Ok(())
    }

    async fn transform(
        &self,
        ctx: &RunContext,
        history: &[Message],
    ) -> HarnessResult<Vec<Message>> {
        ctx.check()?;
        if ctx.scope().binding()? != self.scope {
            return Err(Error::Denied("unified scope".into()));
        }
        Ok(history.to_vec())
    }

    async fn completion(
        &self,
        ctx: &RunContext,
        _history: &[Message],
    ) -> HarnessResult<Option<String>> {
        ctx.check()?;
        Ok(None)
    }
}

/// Writes only under an isolated worktree. Source repo paths are refused.
pub struct IsolatedWriteBoundary {
    scope: String,
    root: PathBuf,
}

impl IsolatedWriteBoundary {
    pub fn new(ctx: &RunContext, root: &Path) -> HarnessResult<Self> {
        if ctx.scope().tenant.is_some() {
            return Err(Error::Denied(
                "enterprise requires live ProductAcl boundary".into(),
            ));
        }
        Ok(Self {
            scope: ctx.scope().binding()?,
            root: std::fs::canonicalize(root)?,
        })
    }
}

#[async_trait]
impl HarnessBoundary for IsolatedWriteBoundary {
    async fn preflight(&self, ctx: &RunContext, call: &Invocation) -> HarnessResult<()> {
        ctx.check()?;
        if ctx.scope().binding()? != self.scope {
            return Err(Error::Denied("write isolation scope".into()));
        }
        if !is_workspace_file_tool(&call.name) {
            return Err(Error::Denied(
                "isolated write child only permits file tools".into(),
            ));
        }
        resolve_under_root(
            &self.root,
            invocation_file_path(call)?,
            "path outside isolated worktree",
        )?;
        Ok(())
    }

    async fn transform(
        &self,
        ctx: &RunContext,
        history: &[Message],
    ) -> HarnessResult<Vec<Message>> {
        ctx.check()?;
        Ok(history.to_vec())
    }

    async fn completion(
        &self,
        ctx: &RunContext,
        _history: &[Message],
    ) -> HarnessResult<Option<String>> {
        ctx.check()?;
        Ok(None)
    }
}

struct FailoverHost {
    inner: AnyCodeHost,
    runtime: Arc<AgentRuntime>,
    task_id: TaskId,
}

fn decode_messages(messages: &[ProviderMessage]) -> HarnessResult<Vec<Message>> {
    messages
        .iter()
        .cloned()
        .map(|v| serde_json::from_value(v).map_err(Error::from))
        .collect()
}

fn hop_from_response(mut response: LLMResponse) -> HarnessResult<Hop> {
    if !response.tool_calls.is_empty() {
        response.message.metadata.insert(
            ANYCODE_TOOL_CALLS_METADATA_KEY.into(),
            serde_json::to_value(&response.tool_calls)?,
        );
    }
    Ok(Hop {
        assistant: serde_json::to_value(response.message)?,
        calls: response
            .tool_calls
            .into_iter()
            .map(|c| Invocation {
                id: c.id,
                name: c.name,
                arguments: c.input,
            })
            .collect(),
        usage: Some(HarnessUsage {
            input_tokens: response.usage.input_tokens as u64,
            output_tokens: response.usage.output_tokens as u64,
            cache_creation_tokens: response.usage.cache_creation_tokens.unwrap_or(0) as u64,
            cache_read_tokens: response.usage.cache_read_tokens.unwrap_or(0) as u64,
        }),
    })
}

#[async_trait]
impl Host for FailoverHost {
    fn tools(&self) -> HarnessResult<Vec<ToolSpec>> {
        self.inner.tools()
    }

    async fn infer(
        &self,
        ctx: &RunContext,
        messages: Vec<ProviderMessage>,
        tools: Vec<ToolSpec>,
    ) -> HarnessResult<Hop> {
        ctx.check()?;
        if self.inner.streaming {
            match self.inner.infer(ctx, messages.clone(), tools.clone()).await {
                Ok(hop) => return Ok(hop),
                Err(Error::Cancelled) => return Err(Error::Cancelled),
                Err(_) => {
                    ctx.check()?;
                }
            }
        }
        let decoded = decode_messages(&messages)?;
        let schemas = tools
            .into_iter()
            .map(|s| ToolSchema {
                name: s.name,
                description: s.description,
                input_schema: s.input_schema,
            })
            .collect();
        let logger = self.runtime.logger();
        let response = self
            .runtime
            .chat_with_failover(&decoded, schemas, &self.inner.model, self.task_id, &logger)
            .await
            .map_err(|e| Error::Host(format!("provider request failed: {e}")))?;
        hop_from_response(response)
    }

    async fn invoke_checked(
        &self,
        ctx: &RunContext,
        call: &Invocation,
    ) -> HarnessResult<ToolResult> {
        self.inner.invoke_checked(ctx, call).await
    }

    fn user_message(&self, text: &str) -> HarnessResult<ProviderMessage> {
        self.inner.user_message(text)
    }

    fn result_message(
        &self,
        call: &Invocation,
        result: &ToolResult,
    ) -> HarnessResult<ProviderMessage> {
        self.inner.result_message(call, result)
    }

    async fn transform_context(
        &self,
        ctx: &RunContext,
        messages: &[ProviderMessage],
    ) -> HarnessResult<Vec<ProviderMessage>> {
        self.inner.transform_context(ctx, messages).await
    }

    async fn accept_completion(
        &self,
        ctx: &RunContext,
        history: &[ProviderMessage],
    ) -> HarnessResult<Option<String>> {
        self.inner.accept_completion(ctx, history).await
    }
}

fn local_scope(session_id: uuid::Uuid, user_id: Option<&str>) -> Scope {
    let subject = user_id
        .and_then(|s| uuid::Uuid::parse_str(s).ok())
        .unwrap_or_else(uuid::Uuid::new_v4);
    Scope {
        subject,
        organization: None,
        tenant: None,
        project: session_id,
        device: None,
    }
}

fn budget_pool_from_task(budget: &TaskBudget) -> HarnessResult<BudgetPool> {
    let limit = u64::from(budget.token_budget_total.unwrap_or(1_000_000)).max(1);
    BudgetPool::new(limit)
}

fn lifetime_from_budget(budget: &TaskBudget) -> Duration {
    Duration::from_secs(budget.max_duration_secs.unwrap_or(3600).max(1))
}

fn limits_from_loop(loop_limits: AgentLoopLimits, token_limit: u64) -> Limits {
    Limits {
        max_turns: loop_limits.max_agent_turns.max(1),
        max_tools_per_turn: 32.min(loop_limits.max_tool_calls.max(1)),
        reservation_per_hop: token_limit.min(32_768).max(1),
        ..Limits::default()
    }
}

fn unified_run_context(
    session_id: uuid::Uuid,
    user_id: Option<&str>,
    bindings: &BTreeMap<String, (String, bool)>,
    budget: &TaskBudget,
) -> HarnessResult<(RunContext, u64)> {
    let caps = bindings
        .values()
        .map(|(cap, _)| cap.clone())
        .collect::<Vec<_>>();
    let token_limit = u64::from(budget.token_budget_total.unwrap_or(1_000_000)).max(1);
    let ctx = RunContext::root(
        local_scope(session_id, user_id),
        Capabilities::new(caps)?,
        budget_pool_from_task(budget)?,
        lifetime_from_budget(budget),
    )?;
    Ok((ctx, token_limit))
}

fn spawn_cancel_bridge(
    ctx: RunContext,
    flag: Option<Arc<AtomicBool>>,
) -> Option<tokio::task::JoinHandle<()>> {
    let flag = flag?;
    Some(tokio::spawn(async move {
        loop {
            if flag.load(Ordering::Acquire) {
                ctx.cancel();
                break;
            }
            if ctx.check().is_err() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    }))
}

fn decode_history(history: &[ProviderMessage]) -> Result<Vec<Message>, CoreError> {
    history
        .iter()
        .cloned()
        .map(|v| serde_json::from_value(v).map_err(CoreError::SerializationError))
        .collect()
}

pub fn journal_kinds(journal: &MemoryJournal) -> Vec<String> {
    journal
        .records()
        .unwrap_or_default()
        .into_iter()
        .map(|r| r.event.kind)
        .collect()
}

#[derive(Clone, Default)]
pub(crate) struct UnifiedLifecycleSpec {
    pub family: Option<TaskFamily>,
    pub gate_plan: Option<GatePlan>,
    pub expected: Vec<ExpectedArtifact>,
}

impl AgentRuntime {
    pub(super) async fn unified_tool_bindings(
        &self,
        llm_names: &[String],
    ) -> HarnessResult<BTreeMap<String, (String, bool)>> {
        let tools = self.tools.read().await;
        let mut bindings = BTreeMap::new();
        for name in llm_names {
            let Some((cap, ro)) = capability_for_legacy_tool(name) else {
                continue;
            };
            if tools.contains_key(name) {
                bindings.insert(name.clone(), (cap, ro));
            }
        }
        if tools.contains_key("Echo") && !bindings.contains_key("Echo") {
            if let Some(binding) = capability_for_legacy_tool("Echo") {
                bindings.insert("Echo".into(), binding);
            }
        }
        if bindings.is_empty() {
            return Err(Error::Invalid("zero harness tool bindings".into()));
        }
        Ok(bindings)
    }

    pub(crate) async fn run_unified_kernel(
        self: &Arc<Self>,
        ctx: &RunContext,
        working_directory: &Path,
        agent: AgentType,
        model: ModelConfig,
        bindings: BTreeMap<String, (String, bool)>,
        history: Vec<Message>,
        journal: &dyn EventSink,
        controls: &ControlQueue,
        limits: Limits,
        streaming: bool,
        task_id: TaskId,
        session_label: &str,
        prompt: &str,
        cancel: Option<Arc<AtomicBool>>,
        lifecycle_spec: UnifiedLifecycleSpec,
        previews: PreviewBus,
    ) -> HarnessResult<(Vec<ProviderMessage>, Vec<Artifact>)> {
        let wd = std::fs::canonicalize(working_directory)?;
        let lifecycle = Arc::new(RunLifecycle {
            family: lifecycle_spec.family,
            gate_plan: lifecycle_spec.gate_plan,
            expected: lifecycle_spec.expected,
            ..RunLifecycle::default()
        });
        let inner_boundary = Arc::new(UnifiedTaskBoundary::new(ctx, &wd, bindings.clone())?);
        let boundary = Arc::new(LifecycleBoundary {
            inner: inner_boundary,
            runtime: self.clone(),
            working_directory: wd.clone(),
            session_label: session_label.to_string(),
            task_id,
            prompt: prompt.to_string(),
            lifecycle: lifecycle.clone(),
        });
        let mut host = self
            .harness_host(
                ctx,
                working_directory,
                agent,
                model,
                bindings,
                boundary,
                previews.clone(),
                lifecycle.clone(),
                super::harness_bridge::HarnessHostPolicy::unified_parent(task_id),
            )
            .await?;
        host.streaming = streaming;
        let host = FailoverHost {
            inner: host,
            runtime: self.clone(),
            task_id,
        };
        let _watch = spawn_cancel_bridge(ctx.clone(), cancel);
        let encoded = history
            .into_iter()
            .map(|m| serde_json::to_value(m).map_err(Error::from))
            .collect::<HarnessResult<Vec<_>>>()?;
        let kernel = Kernel {
            host: &host,
            journal,
            previews,
            controls,
            limits,
        };
        let history = kernel.run(ctx, encoded).await?;
        Ok((history, lifecycle.snapshot_artifacts()))
    }

    pub(super) async fn execute_task_via_unified_kernel(
        &self,
        task: &Task,
        messages: Vec<Message>,
        tool_names: &[String],
        model: ModelConfig,
        loop_limits: AgentLoopLimits,
        lifecycle: UnifiedLifecycleSpec,
    ) -> Result<TaskResult, CoreError> {
        let this = self.upgraded_self()?;
        let logger = self.logger();
        if nested_already_cancelled(&task.context) {
            logger.line(task.id, "[task_end] status=cancelled reason=cancelled");
            return Ok(task_cancelled_failure());
        }
        let bindings = this
            .unified_tool_bindings(tool_names)
            .await
            .map_err(core_from_harness)?;
        let (ctx, token_limit) = unified_run_context(
            task.context.session_id,
            task.context.user_id.as_deref(),
            &bindings,
            &task.context.budget,
        )
        .map_err(core_from_harness)?;
        let journal = MemoryJournal::default();
        let controls = ControlQueue::default();
        let limits = limits_from_loop(loop_limits, token_limit);
        let wd = PathBuf::from(&task.context.working_directory);
        let history = this
            .run_unified_kernel(
                &ctx,
                &wd,
                task.agent_type.clone(),
                model,
                bindings,
                messages,
                &journal,
                &controls,
                limits,
                false,
                task.id,
                &task.context.session_id.to_string(),
                &task.prompt,
                task.context.nested_cancel.clone(),
                lifecycle,
                PreviewBus::default(),
            )
            .await;
        map_task_outcome(&logger, task.id, history)
    }

    pub(super) async fn execute_turn_via_unified_kernel(
        &self,
        task_id: TaskId,
        agent_type: &AgentType,
        messages: Arc<Mutex<Vec<Message>>>,
        working_directory: &str,
        coop_cancel: Option<Arc<AtomicBool>>,
        budget: TaskBudget,
        loop_limits: AgentLoopLimits,
        tool_names: &[String],
        live_trace_tx: Option<tokio::sync::mpsc::UnboundedSender<LiveTraceEvent>>,
        lifecycle: UnifiedLifecycleSpec,
    ) -> Result<TurnOutput, CoreError> {
        let this = self.upgraded_self()?;
        let logger = self.logger();
        if coop_cancel
            .as_ref()
            .is_some_and(|f| f.load(Ordering::Acquire))
        {
            logger.line(task_id, "[task_end] status=cancelled reason=cancelled");
            return Err(CoreError::CooperativeCancel);
        }
        let bindings = this
            .unified_tool_bindings(tool_names)
            .await
            .map_err(core_from_harness)?;
        let session = uuid::Uuid::new_v4();
        let (ctx, token_limit) =
            unified_run_context(session, None, &bindings, &budget).map_err(core_from_harness)?;
        let snapshot = messages.lock().await.clone();
        let prompt = snapshot
            .iter()
            .rev()
            .find_map(|m| match (&m.role, &m.content) {
                (MessageRole::User, MessageContent::Text(t)) => Some(t.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let journal = MemoryJournal::default();
        let controls = ControlQueue::default();
        let limits = limits_from_loop(loop_limits, token_limit);
        let (previews, mut preview_rx) = PreviewBus::bounded(256);
        let preview_tx = live_trace_tx.clone();
        let preview_forward = tokio::spawn(async move {
            while let Some(preview) = preview_rx.recv().await {
                if preview.get("kind").and_then(|v| v.as_str()) == Some("text_delta") {
                    if let Some(text) = preview.get("text").and_then(|v| v.as_str()) {
                        live_trace_emit::emit_assistant_delta(&preview_tx, 1, text, false);
                    }
                }
            }
        });
        let history = this
            .run_unified_kernel(
                &ctx,
                Path::new(working_directory),
                agent_type.clone(),
                self.model_for_task(agent_type).clone(),
                bindings,
                snapshot,
                &journal,
                &controls,
                limits,
                true,
                task_id,
                &session.to_string(),
                &prompt,
                coop_cancel,
                lifecycle,
                previews,
            )
            .await;
        let _ = preview_forward.await;
        match history {
            Ok((encoded, artifacts)) => {
                let decoded = decode_history(&encoded)?;
                let text = last_assistant_plain_text(&decoded).unwrap_or_default();
                *messages.lock().await = decoded;
                logger.line(task_id, "[task_end] status=completed reason=completed");
                Ok(turn_output_from_journal(
                    &journal,
                    text,
                    artifacts,
                    TerminationReason::Completed,
                    "completed",
                    &live_trace_tx,
                ))
            }
            Err(Error::Cancelled) => {
                logger.line(task_id, "[task_end] status=cancelled reason=cancelled");
                emit_unified_workbench_traces(&journal, "", "cancelled", &live_trace_tx);
                Err(CoreError::CooperativeCancel)
            }
            Err(Error::Budget) => Ok(turn_output_from_journal(
                &journal,
                String::new(),
                vec![],
                TerminationReason::Budget,
                "budget",
                &live_trace_tx,
            )),
            Err(Error::Invalid(msg)) if msg.contains("turn limit") => Ok(turn_output_from_journal(
                &journal,
                String::new(),
                vec![],
                TerminationReason::MaxTurns,
                "max_turns",
                &live_trace_tx,
            )),
            Err(Error::Uncertain(msg)) => Ok(turn_output_from_journal(
                &journal,
                msg,
                vec![],
                TerminationReason::Partial,
                "partial",
                &live_trace_tx,
            )),
            Err(err) => {
                emit_unified_workbench_traces(&journal, "", "error", &live_trace_tx);
                Err(CoreError::LLMError(err.to_string()))
            }
        }
    }
}

fn journal_field<'a>(data: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    data.get(key).and_then(|v| v.as_str())
}

fn journal_tool_call(id: &str, name: &str) -> ToolCall {
    ToolCall {
        id: id.to_string(),
        name: name.to_string(),
        input: serde_json::Value::Null,
    }
}

fn turn_output_from_journal(
    journal: &MemoryJournal,
    final_text: String,
    artifacts: Vec<Artifact>,
    termination_reason: TerminationReason,
    status: &str,
    tx: &Option<tokio::sync::mpsc::UnboundedSender<LiveTraceEvent>>,
) -> TurnOutput {
    emit_unified_workbench_traces(journal, &final_text, status, tx);
    TurnOutput {
        final_text,
        artifacts,
        usage: usage_from_journal(journal),
        termination_reason,
    }
}

fn emit_unified_workbench_traces(
    journal: &MemoryJournal,
    final_text: &str,
    status: &str,
    tx: &Option<tokio::sync::mpsc::UnboundedSender<LiveTraceEvent>>,
) {
    live_trace_emit::emit_turn_start(tx, 1);
    let mut tool_idx = 0usize;
    let mut pending_name = String::from("tool");
    if let Ok(records) = journal.records() {
        for record in records {
            match record.event.kind.as_str() {
                "tool_intent" => {
                    let name = journal_field(&record.event.data, "tool").unwrap_or("tool");
                    pending_name = name.to_string();
                    let id = journal_field(&record.event.data, "invocation_id").unwrap_or(name);
                    live_trace_emit::emit_tool_call_start(
                        tx,
                        1,
                        tool_idx,
                        &journal_tool_call(id, name),
                        "",
                    );
                }
                "tool_end" => {
                    let name = pending_name.as_str();
                    let id = journal_field(&record.event.data, "invocation_id").unwrap_or(name);
                    let result = record.event.data.get("result");
                    let value = result
                        .and_then(|v| v.get("value"))
                        .cloned()
                        .unwrap_or(serde_json::Value::Null);
                    let is_error = result
                        .and_then(|v| v.get("is_error"))
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    live_trace_emit::emit_tool_call_end(
                        tx,
                        1,
                        tool_idx,
                        &journal_tool_call(id, name),
                        0,
                        &ToolOutput {
                            result: value,
                            error: is_error.then(|| "tool_error".into()),
                            duration_ms: 0,
                        },
                    );
                    tool_idx = tool_idx.saturating_add(1);
                }
                _ => {}
            }
        }
    }
    live_trace_emit::emit_assistant_done(tx, 1, final_text);
    live_trace_emit::emit_turn_done(tx, status);
}

fn nested_already_cancelled(ctx: &TaskContext) -> bool {
    ctx.nested_cancel
        .as_ref()
        .is_some_and(|b| b.load(Ordering::Acquire))
}

fn core_from_harness(err: Error) -> CoreError {
    match err {
        Error::Cancelled => CoreError::CooperativeCancel,
        other => CoreError::Other(anyhow::anyhow!(other.to_string())),
    }
}

fn map_task_outcome(
    logger: &super::logging::RunLogger,
    task_id: TaskId,
    history: HarnessResult<(Vec<ProviderMessage>, Vec<Artifact>)>,
) -> Result<TaskResult, CoreError> {
    match history {
        Ok((encoded, artifacts)) => {
            let decoded = decode_history(&encoded)?;
            let output = last_assistant_plain_text(&decoded).unwrap_or_default();
            logger.line(task_id, "[task_end] status=completed reason=completed");
            Ok(TaskResult::Success { output, artifacts })
        }
        Err(Error::Cancelled) => {
            logger.line(task_id, "[task_end] status=cancelled reason=cancelled");
            Ok(task_cancelled_failure())
        }
        Err(Error::Budget) => {
            logger.line(task_id, "[task_end] status=failed reason=budget");
            Ok(TaskResult::Failure {
                error: "运行时预算已用尽".to_string(),
                details: Some(TerminationReason::Budget.as_str().to_string()),
            })
        }
        Err(Error::Invalid(msg)) if msg.contains("turn limit") => {
            logger.line(task_id, "[task_end] status=failed reason=max_turns");
            Ok(TaskResult::Failure {
                error: "达到最大模型轮次，任务未完成".to_string(),
                details: Some(TerminationReason::MaxTurns.as_str().to_string()),
            })
        }
        Err(Error::Uncertain(msg)) => {
            logger.line(task_id, "[task_end] status=partial reason=uncertain");
            Ok(TaskResult::Partial {
                success: String::new(),
                remaining: msg,
            })
        }
        Err(Error::Host(msg)) => {
            logger.line(task_id, "[task_end] status=failed reason=error");
            Ok(TaskResult::Failure {
                error: "LLM 调用失败".to_string(),
                details: Some(msg),
            })
        }
        Err(other) => {
            logger.line(task_id, "[task_end] status=failed reason=error");
            Ok(TaskResult::Failure {
                error: other.to_string(),
                details: Some("harness_kernel".to_string()),
            })
        }
    }
}

fn usage_from_journal(journal: &MemoryJournal) -> TurnTokenUsage {
    let mut usage = TurnTokenUsage::default();
    if let Ok(records) = journal.records() {
        for record in records {
            if record.event.kind != "usage" {
                continue;
            }
            let measured = &record.event.data["usage"];
            let input = measured["input_tokens"].as_u64().unwrap_or(0) as u32;
            let output = measured["output_tokens"].as_u64().unwrap_or(0) as u32;
            let cache_creation = measured["cache_creation_tokens"].as_u64().unwrap_or(0) as u32;
            let cache_read = measured["cache_read_tokens"].as_u64().unwrap_or(0) as u32;
            usage.record(&Usage {
                input_tokens: input,
                output_tokens: output,
                cache_creation_tokens: (cache_creation > 0).then_some(cache_creation),
                cache_read_tokens: (cache_read > 0).then_some(cache_read),
            });
        }
    }
    usage
}

#[allow(dead_code)]
pub(super) fn assert_single_kernel_run(journal: &MemoryJournal) {
    let starts = journal_kinds(journal)
        .into_iter()
        .filter(|k| k == "run_start")
        .count();
    debug_assert_eq!(starts, 1);
}

#[allow(dead_code)]
pub(super) fn cancel_error_text() -> &'static str {
    NESTED_TASK_COOPERATIVE_CANCEL_ERROR
}
