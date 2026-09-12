//! Feature-gated migration seam. Existing Workbench and scheduler routes are NOT
//! switched automatically. New code still calls the real legacy security pipeline.
use super::completion_guard::GuardDecision;
use super::harness_unified::FORBIDDEN_UNIFIED_TOOLS;
pub use super::harness_unified::{capability_for_legacy_tool, journal_kinds};
use super::AgentRuntime;
use anycode_core::{
    AgentType, Artifact, ExpectedArtifact, GatePlan, Message, MessageContent, ModelConfig,
    TaskFamily, ToolCall,
};
use anycode_harness_core::{
    digest::bytes_digest,
    events::PreviewBus,
    journal::EventSink,
    kernel::{ControlQueue, Host, Kernel},
    types::{Invocation, Limits, ProviderMessage, ToolResult, ToolSpec},
    Error, Result, RunContext,
};
use anycode_harness_host::{AnyCodeHost, CheckedExecutor};
use async_trait::async_trait;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

#[async_trait]
pub trait HarnessBoundary: Send + Sync {
    /// Mandatory live project/tenant authorization + full input validation + path /
    /// device / approval checks. This executes IN ADDITION TO SecurityLayer.
    async fn preflight(&self, ctx: &RunContext, call: &Invocation) -> Result<()>;
    async fn transform(&self, ctx: &RunContext, history: &[Message]) -> Result<Vec<Message>>;
    async fn completion(&self, ctx: &RunContext, history: &[Message]) -> Result<Option<String>>;
}

/// Shared host state for artifacts, delivery acceptance and completion-guard repair.
#[derive(Default)]
pub struct RunLifecycle {
    pub artifacts: std::sync::Mutex<Vec<Artifact>>,
    pub written_paths: std::sync::Mutex<Vec<String>>,
    pub declared: std::sync::Mutex<std::collections::HashSet<String>>,
    pub family: Option<TaskFamily>,
    pub gate_plan: Option<GatePlan>,
    pub expected: Vec<ExpectedArtifact>,
    pub repairs_used: std::sync::Mutex<u32>,
    pub last_diagnostics: std::sync::Mutex<Option<String>>,
    pub last_failed_gates: std::sync::Mutex<Vec<String>>,
}

impl RunLifecycle {
    pub fn snapshot_artifacts(&self) -> Vec<Artifact> {
        self.artifacts.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

/// Pilot / graph / child hosts keep the default. The unified parent Kernel may
/// expose `Agent` so it routes through Supervisor via `execute_tool_call`.
#[derive(Clone, Copy, Debug, Default)]
pub struct HarnessHostPolicy {
    pub allow_supervisor_agent: bool,
    pub dispatch_task_id: Option<uuid::Uuid>,
    /// Graph work nodes bind FileRead only. The product Workbench runtime is
    /// often not `sandbox_mode`; the security pipeline still applies.
    pub allow_unsandboxed_readonly: bool,
}

impl HarnessHostPolicy {
    #[must_use]
    pub fn unified_parent(task_id: uuid::Uuid) -> Self {
        Self {
            allow_supervisor_agent: true,
            dispatch_task_id: Some(task_id),
            allow_unsandboxed_readonly: false,
        }
    }

    #[must_use]
    pub fn readonly_graph() -> Self {
        Self {
            allow_unsandboxed_readonly: true,
            ..Self::default()
        }
    }

    fn denies_binding(self, name: &str) -> bool {
        FORBIDDEN_UNIFIED_TOOLS.contains(&name) || (name == "Agent" && !self.allow_supervisor_agent)
    }
}

struct RuntimeExecutor {
    runtime: Arc<AgentRuntime>,
    working_directory: PathBuf,
    agent: AgentType,
    boundary: Arc<dyn HarnessBoundary>,
    root_run: uuid::Uuid,
    scope_digest: String,
    allowed: BTreeMap<String, (String, bool)>,
    lifecycle: Arc<RunLifecycle>,
    dispatch_task_id: Option<uuid::Uuid>,
}
#[async_trait]
impl CheckedExecutor for RuntimeExecutor {
    async fn invoke(&self, ctx: &RunContext, call: &Invocation) -> Result<ToolResult> {
        ctx.check()?;
        if ctx.scope().binding()? != self.scope_digest || ctx.root_id() != self.root_run {
            return Err(Error::Denied("host scope/root mismatch".into()));
        }
        let (capability, read_only) = self
            .allowed
            .get(&call.name)
            .ok_or_else(|| Error::Denied("tool outside host registry".into()))?;
        ctx.capabilities().require(capability)?;
        self.boundary.preflight(ctx, call).await?;
        ctx.check()?;
        let input = ToolCall {
            id: call.id.clone(),
            name: call.name.clone(),
            input: call.arguments.clone(),
        };
        let _ctx_guard = self
            .runtime
            .harness_hub
            .enter(ctx.clone(), self.working_directory.clone());
        let dispatch_id = self.dispatch_task_id.unwrap_or_else(|| ctx.id());
        let out = self
            .runtime
            .execute_tool_call(
                dispatch_id,
                &self.agent,
                &self.working_directory.to_string_lossy(),
                &input,
            )
            .await;
        match out {
            Ok(out) => {
                if matches!(input.name.as_str(), "FileWrite" | "Edit" | "NotebookEdit") {
                    if let Some(path) = out
                        .result
                        .get("path")
                        .and_then(|v| v.as_str())
                        .or_else(|| input.input.get("file_path").and_then(|v| v.as_str()))
                        .or_else(|| input.input.get("path").and_then(|v| v.as_str()))
                    {
                        if let Ok(mut written) = self.lifecycle.written_paths.lock() {
                            if !written.iter().any(|p| p == path) {
                                written.push(path.to_string());
                            }
                        }
                    }
                }
                let extracted = super::artifacts::extract_artifacts(&input, &out);
                if let Ok(mut arts) = self.lifecycle.artifacts.lock() {
                    for art in extracted {
                        if let Some(path) = art.path.clone() {
                            if let Ok(mut written) = self.lifecycle.written_paths.lock() {
                                if !written.contains(&path) {
                                    written.push(path);
                                }
                            }
                        }
                        arts.push(art);
                    }
                }
                let declared = self.lifecycle.snapshot_artifacts();
                let mut checked = self
                    .lifecycle
                    .declared
                    .lock()
                    .map(|g| g.clone())
                    .unwrap_or_default();
                let acceptance = self
                    .runtime
                    .check_declared_deliverables(&declared, &mut checked, &self.working_directory)
                    .await;
                if let Ok(mut set) = self.lifecycle.declared.lock() {
                    *set = checked;
                }
                let mut value = out.result;
                if let Some(repair) = acceptance.repair_message() {
                    if let Some(obj) = value.as_object_mut() {
                        obj.insert(
                            "_harness_delivery_repair".into(),
                            serde_json::Value::String(repair),
                        );
                    }
                }
                Ok(ToolResult {
                    value,
                    is_error: out.error.is_some(),
                })
            }
            Err(_) if !*read_only => Err(Error::Uncertain(
                "legacy mutating tool returned an error; inspect effects".into(),
            )),
            Err(_) => Err(Error::Host("legacy read-only tool failed".into())),
        }
    }
    async fn transform(&self, ctx: &RunContext, history: &[Message]) -> Result<Vec<Message>> {
        self.boundary.transform(ctx, history).await
    }
    async fn completion(&self, ctx: &RunContext, history: &[Message]) -> Result<Option<String>> {
        self.boundary.completion(ctx, history).await
    }
}
impl AgentRuntime {
    #[must_use]
    pub fn harness_model_config(&self) -> ModelConfig {
        self.default_model_config.clone()
    }

    /// Build a host from trusted allowlisted tool bindings. This does not register
    /// a second tool registry or bypass the current runtime's tool gating.
    pub async fn harness_host(
        self: &Arc<Self>,
        ctx: &RunContext,
        working_directory: &Path,
        agent: AgentType,
        model: ModelConfig,
        bindings: BTreeMap<String, (String, bool)>,
        boundary: Arc<dyn HarnessBoundary>,
        previews: PreviewBus,
        lifecycle: Arc<RunLifecycle>,
        policy: HarnessHostPolicy,
    ) -> Result<AnyCodeHost> {
        ctx.check()?;
        if !self.sandbox_mode {
            let readonly = bindings.values().all(|(_, read_only)| *read_only);
            if !policy.allow_unsandboxed_readonly || !readonly {
                return Err(Error::Denied("harness pilot requires sandbox_mode".into()));
            }
        }
        if bindings.is_empty() || bindings.len() > 128 {
            return Err(Error::Invalid("harness binding count".into()));
        }
        let wd = std::fs::canonicalize(working_directory)?;
        // Pilot / graph / child hosts must not expose orchestration tools.
        // The unified parent may bind `Agent` so it reaches Supervisor through
        // `execute_tool_call`; it still cannot bind Task / cron / shell control.
        if bindings.keys().any(|n| policy.denies_binding(n)) {
            return Err(Error::Denied(
                "legacy orchestration/control tools are not allowed in harness pilot".into(),
            ));
        }
        let tools = self.tools.read().await;
        let mut registry = vec![];
        for (name, (capability, read_only)) in &bindings {
            ctx.capabilities().require(capability)?;
            let tool = tools
                .get(name)
                .ok_or_else(|| Error::Invalid(format!("unknown registered tool: {name}")))?;
            registry.push(ToolSpec {
                name: name.clone(),
                description: tool.api_tool_description(),
                input_schema: tool.schema(),
                capability: capability.clone(),
                read_only: *read_only,
            });
        }
        drop(tools);
        let model = anycode_llm::apply_anycode_cloud_model_config(model)
            .map_err(|e| Error::Host(e.to_string()))?;
        Ok(AnyCodeHost {
            llm: self.llm_client.clone(),
            model,
            registry,
            streaming: true,
            previews,
            executor: Arc::new(RuntimeExecutor {
                runtime: self.clone(),
                working_directory: wd,
                agent,
                boundary,
                root_run: ctx.root_id(),
                scope_digest: ctx.scope().binding()?,
                allowed: bindings,
                lifecycle,
                dispatch_task_id: policy.dispatch_task_id,
            }),
        })
    }

    /// Single experimental read-only entry. Chat and scheduler stay on the legacy loop
    /// unless the caller explicitly invokes this method after the experiment switch.
    pub async fn run_readonly_pilot(
        self: &Arc<Self>,
        ctx: &RunContext,
        working_directory: &Path,
        prompt: &str,
        journal: &dyn EventSink,
        controls: &ControlQueue,
        limits: Limits,
    ) -> Result<Vec<ProviderMessage>> {
        let mut bindings = BTreeMap::new();
        bindings.insert("FileRead".into(), ("fs.read".into(), true));
        let lifecycle = Arc::new(RunLifecycle::default());
        let boundary = Arc::new(LifecycleBoundary {
            inner: Arc::new(ReadOnlyPilotBoundary::new(ctx, working_directory)?),
            runtime: self.clone(),
            working_directory: std::fs::canonicalize(working_directory)?,
            session_label: "harness-readonly-pilot".into(),
            task_id: ctx.id(),
            prompt: prompt.to_string(),
            lifecycle: lifecycle.clone(),
        });
        let previews = PreviewBus::default();
        let host = self
            .harness_host(
                ctx,
                working_directory,
                AgentType::new("explore"),
                self.default_model_config.clone(),
                bindings,
                boundary,
                previews.clone(),
                lifecycle,
                HarnessHostPolicy::default(),
            )
            .await?;
        let kernel = Kernel {
            host: &host,
            journal,
            previews,
            controls,
            limits,
        };
        kernel.run(ctx, vec![host.user_message(prompt)?]).await
    }
}
/// Conservative local pilot: ONLY FileRead in an immutable host-provisioned root.
/// Not suitable as a multi-tenant authorization implementation. No implicit cloud grant.
pub struct ReadOnlyPilotBoundary {
    scope: String,
    root: PathBuf,
}
impl ReadOnlyPilotBoundary {
    pub fn new(ctx: &RunContext, root: &Path) -> Result<Self> {
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
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadArgs {
    file_path: String,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
}
#[async_trait]
impl HarnessBoundary for ReadOnlyPilotBoundary {
    async fn preflight(&self, ctx: &RunContext, call: &Invocation) -> Result<()> {
        ctx.check()?;
        if ctx.scope().binding()? != self.scope || call.name != "FileRead" {
            return Err(Error::Denied("pilot permits only scoped FileRead".into()));
        }
        let args: ReadArgs = serde_json::from_value(call.arguments.clone())
            .map_err(|_| Error::Invalid("FileRead schema".into()))?;
        if args.offset == Some(0) || args.limit == Some(0) || args.limit.is_some_and(|n| n > 10000)
        {
            return Err(Error::Invalid("read line bounds".into()));
        }
        let p = Path::new(&args.file_path);
        let p = std::fs::canonicalize(if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.root.join(p)
        })?;
        if !p.starts_with(&self.root)
            || !p.is_file()
            || std::fs::metadata(&p)?.len() > 4 * 1024 * 1024
        {
            return Err(Error::Denied("read path/size outside pilot limits".into()));
        }
        Ok(())
    }
    async fn transform(&self, ctx: &RunContext, history: &[Message]) -> Result<Vec<Message>> {
        ctx.check()?;
        if ctx.scope().binding()? != self.scope {
            return Err(Error::Denied("pilot scope".into()));
        }
        Ok(history.to_vec())
    }
    async fn completion(&self, ctx: &RunContext, _history: &[Message]) -> Result<Option<String>> {
        ctx.check()?;
        Ok(None)
    }
}

/// Attaches existing compaction / memory / completion hooks to the Kernel
/// transform and accept_completion seams. It does not start a second loop.
pub struct LifecycleBoundary {
    pub inner: Arc<dyn HarnessBoundary>,
    pub runtime: Arc<AgentRuntime>,
    pub working_directory: PathBuf,
    pub session_label: String,
    pub task_id: anycode_core::TaskId,
    pub prompt: String,
    pub lifecycle: Arc<RunLifecycle>,
}

const HARNESS_COMPACT_BYTES: usize = 512 * 1024;

#[async_trait]
impl HarnessBoundary for LifecycleBoundary {
    async fn preflight(&self, ctx: &RunContext, call: &Invocation) -> Result<()> {
        self.inner.preflight(ctx, call).await
    }

    async fn transform(&self, ctx: &RunContext, history: &[Message]) -> Result<Vec<Message>> {
        let history = self.inner.transform(ctx, history).await?;
        if !self.runtime.auto_compact {
            return Ok(history);
        }
        let bytes = serde_json::to_vec(&history)?.len();
        if bytes <= HARNESS_COMPACT_BYTES {
            return Ok(history);
        }
        let wd = self.working_directory.to_string_lossy();
        let (compacted, _) = self
            .runtime
            .compact_session_messages(&AgentType::new("explore"), &wd, &history, None, true, None)
            .await
            .map_err(|e| Error::Host(format!("compaction failed: {e}")))?;
        Ok(compacted)
    }

    async fn completion(&self, ctx: &RunContext, history: &[Message]) -> Result<Option<String>> {
        if let Some(inner_repair) = self.inner.completion(ctx, history).await? {
            return Ok(Some(inner_repair));
        }
        let output = history
            .iter()
            .rev()
            .find_map(|m| match (&m.role, &m.content) {
                (anycode_core::MessageRole::Assistant, MessageContent::Text(text)) => {
                    Some(text.clone())
                }
                _ => None,
            })
            .unwrap_or_default();
        let artifacts = self.lifecycle.snapshot_artifacts();
        let written = self
            .lifecycle
            .written_paths
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        let repairs_used = self.lifecycle.repairs_used.lock().map(|g| *g).unwrap_or(0);
        let last_diagnostics = self
            .lifecycle
            .last_diagnostics
            .lock()
            .ok()
            .and_then(|g| g.clone());
        let last_failed_gates = self
            .lifecycle
            .last_failed_gates
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        let guard_out = self
            .runtime
            .completion_guard
            .evaluate(
                &self.task_id.to_string(),
                self.lifecycle.family,
                self.lifecycle.gate_plan.as_ref(),
                &self.lifecycle.expected,
                &artifacts,
                &self.working_directory,
                repairs_used,
                last_diagnostics.as_deref(),
                &last_failed_gates,
                &written,
            )
            .await;
        match guard_out.decision {
            GuardDecision::Repair => {
                if let Ok(mut used) = self.lifecycle.repairs_used.lock() {
                    *used = used.saturating_add(1);
                }
                if let Ok(mut diag) = self.lifecycle.last_diagnostics.lock() {
                    *diag = guard_out.repair_message.clone();
                }
                if let Some(report) = &guard_out.report {
                    if let Ok(mut gates) = self.lifecycle.last_failed_gates.lock() {
                        *gates = report
                            .results
                            .iter()
                            .filter(|r| r.outcome == anycode_core::VerificationOutcome::TaskFailed)
                            .map(|r| r.gate_id.clone())
                            .collect();
                    }
                }
                return Ok(guard_out.repair_message);
            }
            GuardDecision::Failed => {
                return Err(Error::Denied(
                    guard_out
                        .repair_message
                        .unwrap_or_else(|| "completion guard failed".into()),
                ));
            }
            GuardDecision::Partial => {
                return Err(Error::Uncertain(
                    guard_out
                        .repair_message
                        .unwrap_or_else(|| "completion guard partial".into()),
                ));
            }
            GuardDecision::Complete => {}
        }
        if !output.is_empty() {
            let prompt = if self.prompt.is_empty() {
                "harness-readonly-pilot"
            } else {
                self.prompt.as_str()
            };
            self.runtime
                .maybe_autosave_memory(self.task_id, prompt, &output)
                .await;
            self.runtime.maybe_session_notify_agent_turn(
                &self.session_label,
                self.task_id,
                0,
                &output,
                Some(&self.working_directory.to_string_lossy()),
            );
        }
        Ok(None)
    }
}

/// Host-owned git worktree location for write children. Storage must sit
/// outside the source repository.
#[derive(Clone, Debug)]
pub struct WriteIsolation {
    pub git: PathBuf,
    pub repository: PathBuf,
    pub private_root: PathBuf,
}

/// Subagent executor that reuses Supervisor + the same Kernel. Write profiles
/// require [`WriteIsolation`] and never share the parent workspace.
pub struct KernelChildExecutor {
    runtime: Arc<AgentRuntime>,
    working_directory: PathBuf,
    journal: Arc<dyn EventSink>,
    limits: Limits,
    write_isolation: Option<WriteIsolation>,
}

const READ_ONLY_CHILD_AGENTS: &[&str] = &["explore", "planner", "plan", "reviewer"];

impl KernelChildExecutor {
    pub fn new(
        runtime: Arc<AgentRuntime>,
        working_directory: PathBuf,
        journal: Arc<dyn EventSink>,
        limits: Limits,
    ) -> Self {
        Self {
            runtime,
            working_directory,
            journal,
            limits,
            write_isolation: None,
        }
    }

    #[must_use]
    pub fn with_write_isolation(mut self, isolation: WriteIsolation) -> Self {
        self.write_isolation = Some(isolation);
        self
    }

    async fn execute_isolated_write(
        &self,
        ctx: RunContext,
        assignment: anycode_harness_extensions::subagents::Assignment,
    ) -> Result<anycode_harness_extensions::subagents::ChildOutcome> {
        if assignment.agent != "implementer" {
            return Err(Error::Denied(
                "write child must use the implementer profile".into(),
            ));
        }
        if assignment.capabilities.iter().any(|cap| {
            cap != "fs.read" && cap != "fs.write" && cap != "skill.read" && cap != "worktree.create"
        }) {
            return Err(Error::Denied(
                "write child capabilities are limited to file/worktree".into(),
            ));
        }
        let isolation = self.write_isolation.as_ref().ok_or_else(|| {
            Error::Denied("write/computer children require an isolated worktree host".into())
        })?;
        ctx.capabilities().require("worktree.create")?;
        ctx.capabilities().require("fs.write")?;
        let lease = anycode_harness_extensions::worktree::create(
            &ctx,
            &isolation.git,
            &isolation.repository,
            &isolation.private_root,
        )
        .await?;
        let mut bindings = BTreeMap::new();
        if ctx.capabilities().allows("fs.read") {
            bindings.insert("FileRead".into(), ("fs.read".into(), true));
        }
        bindings.insert("FileWrite".into(), ("fs.write".into(), false));
        let lifecycle = Arc::new(RunLifecycle::default());
        let boundary = Arc::new(LifecycleBoundary {
            inner: Arc::new(super::harness_unified::IsolatedWriteBoundary::new(
                &ctx,
                &lease.path,
            )?),
            runtime: self.runtime.clone(),
            working_directory: std::fs::canonicalize(&lease.path)?,
            session_label: "harness-write-child".into(),
            task_id: ctx.id(),
            prompt: assignment.prompt.clone(),
            lifecycle: lifecycle.clone(),
        });
        let previews = PreviewBus::default();
        let host = self
            .runtime
            .harness_host(
                &ctx,
                &lease.path,
                AgentType::new("general-purpose"),
                self.runtime.default_model_config.clone(),
                bindings,
                boundary,
                previews.clone(),
                lifecycle,
                HarnessHostPolicy::default(),
            )
            .await?;
        let controls = ControlQueue::default();
        let kernel = Kernel {
            host: &host,
            journal: self.journal.as_ref(),
            previews,
            controls: &controls,
            limits: self.limits.clone(),
        };
        let history = kernel
            .run(&ctx, vec![host.user_message(&assignment.prompt)?])
            .await?;
        child_text_outcome(&ctx, history)
    }
}

fn child_text_outcome(
    ctx: &RunContext,
    history: Vec<anycode_harness_core::types::ProviderMessage>,
) -> Result<anycode_harness_extensions::subagents::ChildOutcome> {
    let last: Message = serde_json::from_value(
        history
            .last()
            .cloned()
            .ok_or_else(|| Error::Host("empty child history".into()))?,
    )?;
    let MessageContent::Text(text) = last.content else {
        return Err(Error::Host("child final answer is not text".into()));
    };
    Ok(anycode_harness_extensions::subagents::ChildOutcome {
        run_id: ctx.id(),
        output: serde_json::json!({"text": text}),
        partial: false,
    })
}

#[async_trait]
impl anycode_harness_extensions::subagents::ChildExecutor for KernelChildExecutor {
    async fn execute(
        &self,
        ctx: RunContext,
        assignment: anycode_harness_extensions::subagents::Assignment,
    ) -> Result<anycode_harness_extensions::subagents::ChildOutcome> {
        ctx.check()?;
        let wants_write = assignment.capabilities.iter().any(|cap| cap == "fs.write");
        if wants_write {
            return self.execute_isolated_write(ctx, assignment).await;
        }
        if !READ_ONLY_CHILD_AGENTS.contains(&assignment.agent.as_str()) {
            return Err(Error::Denied(
                "child profile is not on the trusted read-only allowlist".into(),
            ));
        }
        if assignment
            .capabilities
            .iter()
            .any(|cap| cap != "fs.read" && cap != "skill.read")
        {
            return Err(Error::Denied(
                "write/computer children require an isolated worktree host".into(),
            ));
        }
        let controls = ControlQueue::default();
        let history = self
            .runtime
            .run_readonly_pilot(
                &ctx,
                &self.working_directory,
                &assignment.prompt,
                self.journal.as_ref(),
                &controls,
                self.limits.clone(),
            )
            .await?;
        child_text_outcome(&ctx, history)
    }
}

/// Trusted graph factory: only read-only profiles, and verifiers fail closed.
pub struct RuntimeHostFactory {
    runtime: Arc<AgentRuntime>,
    working_directory: PathBuf,
    previews: PreviewBus,
}

impl RuntimeHostFactory {
    pub fn new(
        runtime: Arc<AgentRuntime>,
        working_directory: PathBuf,
        previews: PreviewBus,
    ) -> Self {
        Self {
            runtime,
            working_directory,
            previews,
        }
    }
}

#[async_trait]
impl anycode_harness_host::graph_adapter::AgentHostFactory for RuntimeHostFactory {
    async fn build(
        &self,
        ctx: &RunContext,
        agent: &str,
    ) -> Result<anycode_harness_host::AnyCodeHost> {
        if !READ_ONLY_CHILD_AGENTS.contains(&agent) {
            return Err(Error::Denied(
                "graph agent profile is not on the trusted read-only allowlist".into(),
            ));
        }
        let mut bindings = BTreeMap::new();
        bindings.insert("FileRead".into(), ("fs.read".into(), true));
        let lifecycle = Arc::new(RunLifecycle::default());
        let boundary = Arc::new(LifecycleBoundary {
            inner: Arc::new(ReadOnlyPilotBoundary::new(ctx, &self.working_directory)?),
            runtime: self.runtime.clone(),
            working_directory: std::fs::canonicalize(&self.working_directory)?,
            session_label: "harness-graph".into(),
            task_id: ctx.id(),
            prompt: agent.to_string(),
            lifecycle: lifecycle.clone(),
        });
        self.runtime
            .harness_host(
                ctx,
                &self.working_directory,
                AgentType::new(agent),
                self.runtime.default_model_config.clone(),
                bindings,
                boundary,
                self.previews.clone(),
                lifecycle,
                HarnessHostPolicy::readonly_graph(),
            )
            .await
    }

    async fn verify(
        &self,
        ctx: &RunContext,
        verifier: &str,
        inputs: BTreeMap<String, serde_json::Value>,
    ) -> Result<anycode_harness_extensions::graph::Verification> {
        ctx.check()?;
        verify_trusted_artifact(&self.working_directory, verifier, &inputs)
    }
}

/// Host-owned evidence hash. Sample verifier IDs stay unsupported.
pub fn verify_trusted_artifact(
    root: &Path,
    verifier: &str,
    inputs: &BTreeMap<String, serde_json::Value>,
) -> Result<anycode_harness_extensions::graph::Verification> {
    if verifier != "artifact.sha256" {
        return Err(Error::Unsupported(
            "sample verifier IDs are not trusted implementations".into(),
        ));
    }
    let root = std::fs::canonicalize(root)?;
    let configured = root.join(".anycode").join("harness").join("evidence");
    let requested = inputs
        .values()
        .find_map(|v| v.get("path").and_then(|p| p.as_str()))
        .map(PathBuf::from);
    let path = requested.unwrap_or(configured);
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    let actual = std::fs::canonicalize(&path)
        .map_err(|_| Error::Denied("gate evidence missing or outside project root".into()))?;
    if !actual.starts_with(&root) || !actual.is_file() {
        return Err(Error::Denied(
            "gate evidence missing or outside project root".into(),
        ));
    }
    if std::fs::symlink_metadata(&actual)?.file_type().is_symlink() {
        return Err(Error::Denied("gate evidence symlink".into()));
    }
    let bytes = std::fs::read(&actual)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(Error::Invalid("gate evidence too large".into()));
    }
    let digest = bytes_digest(&bytes);
    let report_dir = root.join(".anycode").join("harness").join("reports");
    std::fs::create_dir_all(&report_dir)?;
    let report = serde_json::json!({
        "verifier": "artifact.sha256",
        "path": actual.to_string_lossy(),
        "bytes": bytes.len(),
        "artifact_digest": digest,
    });
    std::fs::write(
        report_dir.join(format!("{digest}.json")),
        serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(anycode_harness_extensions::graph::Verification {
        passed: true,
        artifact_digest: digest,
        report,
    })
}

/// Host policy allowlist only. SKILL.md `allowed-tools` never grants capabilities.
pub fn harness_skill_allowlist(
    governance: &anycode_tools::SkillsGovernance,
    agent: &str,
) -> std::collections::BTreeSet<String> {
    governance
        .effective_ids(agent)
        .unwrap_or_default()
        .into_iter()
        .collect()
}
