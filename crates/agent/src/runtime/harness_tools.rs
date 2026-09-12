//! Discoverable harness tools. They fail closed unless the Host entered a RunContext.
use super::harness_bridge::{harness_skill_allowlist, KernelChildExecutor, WriteIsolation};
use super::AgentRuntime;
use anycode_core::prelude::*;
use anycode_harness_core::approval::{ApprovalTicket, ApprovalVault};
use anycode_harness_core::RunContext;
use anycode_harness_extensions::computer::{Action, ComputerBackend, ComputerBroker, Lease};
use anycode_harness_extensions::skills::Catalog;
use anycode_harness_extensions::subagents::{Assignment, ChildOutcome, Supervisor};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, Weak};
use std::time::{Duration, Instant};
use uuid::Uuid;

#[derive(Clone)]
pub struct ComputerTicket {
    pub ticket: Uuid,
    pub device: Uuid,
    pub run: Uuid,
    pub expires: Instant,
}

struct ChildSlot {
    ctx: RunContext,
    join: Option<tokio::task::JoinHandle<anycode_harness_core::Result<ChildOutcome>>>,
    outcome: Option<ChildOutcome>,
}

struct Active {
    ctx: RunContext,
    working_directory: PathBuf,
}

struct EnrolledComputer {
    device: Uuid,
    kind: String,
    broker: Arc<ComputerBroker>,
}

struct ComputerSession {
    lease: Lease,
    frame_id: Uuid,
}

#[derive(Default)]
pub struct HarnessHub {
    current: Mutex<Vec<Active>>,
    supervisors: Mutex<HashMap<Uuid, Arc<Supervisor>>>,
    children: Mutex<HashMap<Uuid, ChildSlot>>,
    tickets: Mutex<HashMap<Uuid, ComputerTicket>>,
    write_isolation: Mutex<Option<WriteIsolation>>,
    computer: Mutex<Option<EnrolledComputer>>,
    computer_session: Mutex<Option<ComputerSession>>,
    action_approvals: ApprovalVault,
    pending_action: Mutex<Option<ApprovalTicket>>,
    skill_digests: Mutex<HashMap<String, String>>,
    bound_product_runs: Mutex<HashSet<Uuid>>,
}

pub struct HubEnterGuard {
    hub: Arc<HarnessHub>,
}

impl Drop for HubEnterGuard {
    fn drop(&mut self) {
        if let Ok(mut stack) = self.hub.current.lock() {
            stack.pop();
        }
    }
}

impl HarnessHub {
    pub fn enter(self: &Arc<Self>, ctx: RunContext, working_directory: PathBuf) -> HubEnterGuard {
        if let Ok(mut stack) = self.current.lock() {
            stack.push(Active {
                ctx,
                working_directory,
            });
        }
        HubEnterGuard { hub: self.clone() }
    }

    pub(crate) fn current(&self) -> anycode_harness_core::Result<(RunContext, PathBuf)> {
        self.current
            .lock()
            .ok()
            .and_then(|s| {
                s.last()
                    .map(|a| (a.ctx.clone(), a.working_directory.clone()))
            })
            .ok_or_else(|| {
                anycode_harness_core::Error::Denied(
                    "harness tool requires an active Host context".into(),
                )
            })
    }

    pub fn issue_computer_ticket(
        &self,
        device: Uuid,
        ttl_secs: u64,
    ) -> anycode_harness_core::Result<ComputerTicket> {
        let (ctx, _) = self.current()?;
        self.issue_computer_ticket_for_run(device, ttl_secs, ctx.id())
    }

    pub fn issue_computer_ticket_for_run(
        &self,
        device: Uuid,
        ttl_secs: u64,
        run: Uuid,
    ) -> anycode_harness_core::Result<ComputerTicket> {
        if device.is_nil() || run.is_nil() || ttl_secs == 0 || ttl_secs > 300 {
            return Err(anycode_harness_core::Error::Invalid(
                "computer ticket bounds".into(),
            ));
        }
        let (ctx, _) = self.current()?;
        if ctx.scope().device != Some(device) {
            return Err(anycode_harness_core::Error::Denied(
                "computer ticket device is not in the current scope".into(),
            ));
        }
        let ticket = ComputerTicket {
            ticket: Uuid::new_v4(),
            device,
            run,
            expires: Instant::now() + Duration::from_secs(ttl_secs),
        };
        self.tickets
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("ticket lock".into()))?
            .insert(ticket.ticket, ticket.clone());
        self.bind_product_run(run)?;
        Ok(ticket)
    }

    pub fn bind_product_run(&self, run: Uuid) -> anycode_harness_core::Result<()> {
        if run.is_nil() {
            return Err(anycode_harness_core::Error::Invalid(
                "product run id".into(),
            ));
        }
        self.bound_product_runs
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("product run lock".into()))?
            .insert(run);
        Ok(())
    }

    pub fn unbind_product_run(&self, run: Uuid) {
        if let Ok(mut runs) = self.bound_product_runs.lock() {
            runs.remove(&run);
        }
    }

    fn consume_ticket(
        &self,
        ticket: Uuid,
        ctx_run: Uuid,
        device: Uuid,
    ) -> anycode_harness_core::Result<()> {
        let mut tickets = self
            .tickets
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("ticket lock".into()))?;
        let issued = tickets
            .remove(&ticket)
            .ok_or_else(|| anycode_harness_core::Error::Denied("computer ticket missing".into()))?;
        let bound = self
            .bound_product_runs
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("product run lock".into()))?;
        let run_ok = issued.run == ctx_run || bound.contains(&issued.run);
        if !run_ok || issued.device != device || issued.expires <= Instant::now() {
            return Err(anycode_harness_core::Error::Denied(
                "computer ticket mismatch or expired".into(),
            ));
        }
        Ok(())
    }

    pub fn set_write_isolation(&self, isolation: WriteIsolation) {
        if let Ok(mut slot) = self.write_isolation.lock() {
            *slot = Some(isolation);
        }
    }

    pub(crate) fn write_isolation(&self) -> Option<WriteIsolation> {
        self.write_isolation.lock().ok().and_then(|g| g.clone())
    }

    pub fn enroll_computer(
        &self,
        device: Uuid,
        kind: &str,
        backend: Arc<dyn ComputerBackend>,
    ) -> anycode_harness_core::Result<()> {
        if device.is_nil() || kind.is_empty() || kind.len() > 64 {
            return Err(anycode_harness_core::Error::Invalid(
                "computer enrollment".into(),
            ));
        }
        let broker = ComputerBroker::new(device, backend)?;
        let mut slot = self
            .computer
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("computer lock".into()))?;
        *slot = Some(EnrolledComputer {
            device,
            kind: kind.to_string(),
            broker: Arc::new(broker),
        });
        Ok(())
    }

    pub fn computer_backend_kind(&self) -> Option<String> {
        self.computer
            .lock()
            .ok()
            .and_then(|g| g.as_ref().map(|e| e.kind.clone()))
    }

    /// HOST ONLY after a trusted UI approver confirms a bound action on the
    /// last observed frame. The model never receives this ticket.
    pub fn approve_computer_action(
        &self,
        frame_id: Uuid,
        action: Action,
    ) -> anycode_harness_core::Result<()> {
        let (ctx, _) = self.current()?;
        let enrolled = self.enrolled_broker()?;
        let session = self
            .computer_session
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("computer session lock".into()))?;
        let session = session.as_ref().ok_or_else(|| {
            anycode_harness_core::Error::Denied("observe before approving an action".into())
        })?;
        if session.frame_id != frame_id {
            return Err(anycode_harness_core::Error::Denied(
                "approval frame does not match the last observation".into(),
            ));
        }
        let binding = enrolled.approval_binding(&ctx, &session.lease, frame_id, &action)?;
        let ticket = self
            .action_approvals
            .issue_from_trusted_ui(binding, Duration::from_secs(30))?;
        *self
            .pending_action
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("approval lock".into()))? = Some(ticket);
        Ok(())
    }

    fn enrolled_broker(&self) -> anycode_harness_core::Result<Arc<ComputerBroker>> {
        self.computer
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("computer lock".into()))?
            .as_ref()
            .map(|enrolled| enrolled.broker.clone())
            .ok_or_else(|| {
                anycode_harness_core::Error::Denied("computer backend is not enrolled".into())
            })
    }

    fn store_computer_session(&self, session: ComputerSession) -> anycode_harness_core::Result<()> {
        *self
            .computer_session
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("computer session lock".into()))? =
            Some(session);
        Ok(())
    }

    fn take_computer_session(&self) -> anycode_harness_core::Result<ComputerSession> {
        self.computer_session
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("computer session lock".into()))?
            .take()
            .ok_or_else(|| anycode_harness_core::Error::Denied("observe before acting".into()))
    }

    fn take_pending_action(&self) -> anycode_harness_core::Result<ApprovalTicket> {
        self.pending_action
            .lock()
            .map_err(|_| anycode_harness_core::Error::Host("approval lock".into()))?
            .take()
            .ok_or_else(|| {
                anycode_harness_core::Error::Denied(
                    "computer act requires a host-issued UI approval for this frame and action"
                        .into(),
                )
            })
    }
}

pub(super) async fn register_harness_tools(runtime: &Arc<AgentRuntime>) {
    let weak = Arc::downgrade(runtime);
    let mut tools = runtime.tools.write().await;
    for (name, tool) in harness_tool_boxes(weak) {
        tools.entry(name).or_insert(tool);
    }
}

fn harness_tool_boxes(runtime: Weak<AgentRuntime>) -> Vec<(ToolName, Box<dyn Tool>)> {
    [
        "HarnessAgentSpawn",
        "HarnessAgentJoin",
        "HarnessAgentCancel",
        "HarnessComputerObserve",
        "HarnessComputerAct",
        "HarnessSkillSearch",
        "HarnessSkillActivate",
    ]
    .into_iter()
    .map(|name| {
        (
            name.into(),
            Box::new(NamedTool {
                name,
                runtime: runtime.clone(),
            }) as Box<dyn Tool>,
        )
    })
    .collect()
}

struct NamedTool {
    name: &'static str,
    runtime: Weak<AgentRuntime>,
}

#[derive(Deserialize)]
struct SpawnArgs {
    agent: String,
    prompt: String,
    #[serde(default)]
    capabilities: Vec<String>,
}

#[derive(Deserialize)]
struct ChildArgs {
    child_run_id: Uuid,
}

#[derive(Deserialize)]
struct TicketArgs {
    ticket: Uuid,
}

#[derive(Deserialize)]
struct SkillArgs {
    name: String,
}

#[async_trait]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        match self.name {
            "HarnessAgentSpawn" => "Spawn a Supervisor child on the same Kernel",
            "HarnessAgentJoin" => "Join a previously spawned harness child",
            "HarnessAgentCancel" => "Cancel a harness child run",
            "HarnessComputerObserve" => "Observe a host-leased computer device",
            "HarnessComputerAct" => "Act on a host-leased computer device",
            "HarnessSkillSearch" => {
                "Search host-allowlisted skills; markdown never grants capabilities"
            }
            "HarnessSkillActivate" => "Activate an allowlisted skill as untrusted instructions",
            _ => "harness tool",
        }
    }

    fn schema(&self) -> serde_json::Value {
        match self.name {
            "HarnessAgentSpawn" => json!({
                "type": "object",
                "properties": {
                    "agent": {"type": "string"},
                    "prompt": {"type": "string"},
                    "capabilities": {"type": "array", "items": {"type": "string"}}
                },
                "required": ["agent", "prompt"]
            }),
            "HarnessAgentJoin" | "HarnessAgentCancel" => json!({
                "type": "object",
                "properties": {"child_run_id": {"type": "string"}},
                "required": ["child_run_id"]
            }),
            "HarnessComputerObserve" => json!({
                "type": "object",
                "properties": {"ticket": {"type": "string"}},
                "required": ["ticket"]
            }),
            "HarnessComputerAct" => json!({
                "type": "object",
                "properties": {
                    "ticket": {"type": "string"},
                    "frame_id": {"type": "string"},
                    "action": {
                        "type": "object",
                        "oneOf": [
                            {"properties": {"type": {"const": "click"}, "x": {"type": "integer"}, "y": {"type": "integer"}}, "required": ["type", "x", "y"]},
                            {"properties": {"type": {"const": "type"}, "text": {"type": "string"}}, "required": ["type", "text"]},
                            {"properties": {"type": {"const": "key"}, "key": {"type": "string"}}, "required": ["type", "key"]},
                            {"properties": {"type": {"const": "scroll"}, "down": {"type": "boolean"}, "steps": {"type": "integer"}}, "required": ["type", "down", "steps"]}
                        ]
                    }
                },
                "required": ["ticket", "frame_id", "action"]
            }),
            "HarnessSkillSearch" => json!({
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"]
            }),
            _ => json!({
                "type": "object",
                "properties": {"name": {"type": "string"}},
                "required": ["name"]
            }),
        }
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let runtime = self
            .runtime
            .upgrade()
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("harness runtime dropped")))?;
        let hub = runtime.harness_hub.clone();
        let value = match self.name {
            "HarnessAgentSpawn" => spawn_child(&runtime, &hub, input.input).await?,
            "HarnessAgentJoin" => join_child(&hub, input.input).await?,
            "HarnessAgentCancel" => cancel_child(&hub, input.input)?,
            "HarnessComputerObserve" => computer_observe(&hub, input.input).await?,
            "HarnessComputerAct" => computer_act(&hub, input.input).await?,
            "HarnessSkillSearch" => search_skills(&runtime, &hub, &input)?,
            "HarnessSkillActivate" => activate_skill(&runtime, &hub, &input)?,
            _ => json!({"error": "unknown harness tool"}),
        };
        Ok(ToolOutput {
            result: value,
            error: None,
            duration_ms: 1,
        })
    }
}

async fn spawn_child(
    runtime: &Arc<AgentRuntime>,
    hub: &Arc<HarnessHub>,
    input: serde_json::Value,
) -> Result<serde_json::Value, CoreError> {
    let args: SpawnArgs = serde_json::from_value(input).map_err(CoreError::SerializationError)?;
    let (parent, working_directory) = hub.current().map_err(hub_err)?;
    parent
        .capabilities()
        .require("agent.spawn")
        .map_err(hub_err)?;
    let caps = if args.capabilities.is_empty() {
        vec!["fs.read".into()]
    } else {
        args.capabilities
    };
    if caps.iter().any(|c| c == "agent.spawn") {
        return Err(CoreError::Other(anyhow::anyhow!(
            "children cannot inherit agent.spawn"
        )));
    }
    let assignment = Assignment {
        agent: args.agent,
        prompt: args.prompt,
        capabilities: anycode_harness_core::Capabilities::new(caps).map_err(hub_err)?,
    };
    let supervisor = {
        let mut guards = hub
            .supervisors
            .lock()
            .map_err(|_| CoreError::Other(anyhow::anyhow!("supervisor lock")))?;
        guards
            .entry(parent.root_id())
            .or_insert_with(|| Arc::new(Supervisor::new(&parent, 1, 4).expect("supervisor")))
            .clone()
    };
    let isolation = hub.write_isolation();
    let journal = Arc::new(anycode_harness_core::journal::MemoryJournal::default());
    let mut executor = KernelChildExecutor::new(
        runtime.clone(),
        working_directory,
        journal,
        child_kernel_limits(&parent),
    );
    if let Some(isolation) = isolation {
        executor = executor.with_write_isolation(isolation);
    }
    let (permit, child) = supervisor.prepare(&parent, &assignment).map_err(hub_err)?;
    let child_id = child.id();
    let join = tokio::spawn({
        let supervisor = supervisor.clone();
        let child = child.clone();
        async move {
            supervisor
                .run_prepared(permit, child, assignment, &executor)
                .await
        }
    });
    hub.children
        .lock()
        .map_err(|_| CoreError::Other(anyhow::anyhow!("child lock")))?
        .insert(
            child_id,
            ChildSlot {
                ctx: child,
                join: Some(join),
                outcome: None,
            },
        );
    runtime.maybe_session_notify_agent_turn(
        "harness-child",
        child_id,
        0,
        "harness_child_spawned",
        None,
    );
    Ok(json!({"child_run_id": child_id}))
}

async fn join_child(
    hub: &Arc<HarnessHub>,
    input: serde_json::Value,
) -> Result<serde_json::Value, CoreError> {
    let args: ChildArgs = serde_json::from_value(input).map_err(CoreError::SerializationError)?;
    let handle = {
        let mut children = hub
            .children
            .lock()
            .map_err(|_| CoreError::Other(anyhow::anyhow!("child lock")))?;
        let slot = children
            .get_mut(&args.child_run_id)
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("unknown child")))?;
        slot.join
            .take()
            .ok_or_else(|| CoreError::Other(anyhow::anyhow!("child already joined")))?
    };
    let outcome = handle
        .await
        .map_err(|e| CoreError::Other(anyhow::anyhow!(e.to_string())))?
        .map_err(hub_err)?;
    if let Ok(mut children) = hub.children.lock() {
        if let Some(slot) = children.get_mut(&args.child_run_id) {
            slot.outcome = Some(outcome.clone());
        }
    }
    Ok(outcome.output)
}

fn cancel_child(
    hub: &Arc<HarnessHub>,
    input: serde_json::Value,
) -> Result<serde_json::Value, CoreError> {
    let args: ChildArgs = serde_json::from_value(input).map_err(CoreError::SerializationError)?;
    let children = hub
        .children
        .lock()
        .map_err(|_| CoreError::Other(anyhow::anyhow!("child lock")))?;
    let slot = children
        .get(&args.child_run_id)
        .ok_or_else(|| CoreError::Other(anyhow::anyhow!("unknown child")))?;
    slot.ctx.cancel();
    Ok(json!({"cancelled": true, "child_run_id": args.child_run_id}))
}

async fn computer_observe(
    hub: &Arc<HarnessHub>,
    input: serde_json::Value,
) -> Result<serde_json::Value, CoreError> {
    let args: TicketArgs = serde_json::from_value(input).map_err(CoreError::SerializationError)?;
    let (ctx, working_directory) = hub.current().map_err(hub_err)?;
    let device = consume_scoped_ticket(hub, &ctx, args.ticket)?;
    ctx.capabilities()
        .require("computer.observe")
        .map_err(hub_err)?;
    let (broker, kind) = enrolled_computer(hub, device)?;
    let mut lease = broker
        .acquire(&ctx, Duration::from_secs(30))
        .await
        .map_err(hub_err)?;
    let obs = broker.observe(&ctx, &mut lease).await.map_err(hub_err)?;
    let dir = working_directory
        .join(".anycode")
        .join("harness")
        .join("computer");
    std::fs::create_dir_all(&dir).map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?;
    let path = dir.join(format!("{}.png", obs.frame_id));
    if path.exists() {
        return Err(CoreError::Other(anyhow::anyhow!(
            "computer frame path collision"
        )));
    }
    std::fs::write(&path, &obs.png).map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?;
    hub.store_computer_session(ComputerSession {
        lease,
        frame_id: obs.frame_id,
    })
    .map_err(hub_err)?;
    Ok(json!({
        "frame_id": obs.frame_id,
        "width": obs.width,
        "height": obs.height,
        "target": obs.target,
        "path": path.to_string_lossy(),
        "bytes": obs.png.len(),
        "private": true,
        "backend": kind,
        "note": "screenshot is a private host artifact; it is not written to the journal"
    }))
}

#[derive(Deserialize)]
struct ActArgs {
    ticket: Uuid,
    frame_id: Uuid,
    action: Action,
}

async fn computer_act(
    hub: &Arc<HarnessHub>,
    input: serde_json::Value,
) -> Result<serde_json::Value, CoreError> {
    let args: ActArgs = serde_json::from_value(input).map_err(CoreError::SerializationError)?;
    let (ctx, _) = hub.current().map_err(hub_err)?;
    let device = consume_scoped_ticket(hub, &ctx, args.ticket)?;
    ctx.capabilities()
        .require("computer.input")
        .map_err(hub_err)?;
    let (broker, kind) = enrolled_computer(hub, device)?;
    let approval = hub.take_pending_action().map_err(hub_err)?;
    let mut session = hub.take_computer_session().map_err(hub_err)?;
    if session.frame_id != args.frame_id {
        let _ = hub.store_computer_session(session);
        return Err(CoreError::Other(anyhow::anyhow!(
            "act frame does not match the last host observation"
        )));
    }
    broker
        .act(
            &ctx,
            &mut session.lease,
            args.frame_id,
            args.action,
            approval,
            &hub.action_approvals,
        )
        .await
        .map_err(hub_err)?;
    Ok(json!({
        "acted": true,
        "backend": kind,
        "frame_id": args.frame_id,
        "note": "macOS ScreenCapture remains observe-only; X11/Windows input is unverified"
    }))
}

#[derive(Deserialize)]
struct SearchArgs {
    query: String,
}

fn skill_governance(
    runtime: &AgentRuntime,
    working_directory: &Path,
    skill_hint: Option<&str>,
) -> Result<(BTreeSet<String>, Catalog), CoreError> {
    let services = runtime
        .tool_services
        .lock()
        .ok()
        .and_then(|g| g.clone())
        .ok_or_else(|| CoreError::Other(anyhow::anyhow!("tool services missing")))?;
    let gov = services
        .skills_governance
        .lock()
        .map_err(|_| CoreError::Other(anyhow::anyhow!("skills governance lock")))?;
    let allowlist = harness_skill_allowlist(&gov, "general-purpose");
    drop(gov);
    let mut roots = services.skill_catalog.roots_scanned.clone();
    if roots.is_empty() {
        if let Some(name) = skill_hint {
            if let Some(root) = services
                .skill_catalog
                .resolve_skill_root(name, Some(working_directory))
            {
                if let Some(parent) = root.parent() {
                    roots.push(parent.to_path_buf());
                }
            }
        }
    }
    if roots.is_empty() {
        let local = working_directory.join(".anycode").join("skills");
        if local.is_dir() {
            roots.push(local);
        }
    }
    if roots.is_empty() {
        return Err(CoreError::Other(anyhow::anyhow!(
            "no host-managed skill roots"
        )));
    }
    let catalog = Catalog::discover(&roots).map_err(hub_err)?;
    Ok((allowlist, catalog))
}

fn search_skills(
    runtime: &AgentRuntime,
    hub: &Arc<HarnessHub>,
    input: &ToolInput,
) -> Result<serde_json::Value, CoreError> {
    let args: SearchArgs =
        serde_json::from_value(input.input.clone()).map_err(CoreError::SerializationError)?;
    let (ctx, working_directory) = hub.current().map_err(hub_err)?;
    ctx.capabilities().require("skill.read").map_err(hub_err)?;
    let (allowlist, catalog) = skill_governance(runtime, &working_directory, None)?;
    let matches = catalog.search(&args.query, &allowlist, 20);
    if let Ok(mut seen) = hub.skill_digests.lock() {
        for item in &matches {
            seen.insert(item.name.clone(), item.sha256.clone());
        }
    }
    Ok(json!({
        "matches": matches,
        "untrusted": true,
        "note": "SKILL.md allowed-tools never grants capabilities"
    }))
}

fn activate_skill(
    runtime: &AgentRuntime,
    hub: &Arc<HarnessHub>,
    input: &ToolInput,
) -> Result<serde_json::Value, CoreError> {
    let args: SkillArgs =
        serde_json::from_value(input.input.clone()).map_err(CoreError::SerializationError)?;
    let (ctx, working_directory) = hub.current().map_err(hub_err)?;
    ctx.capabilities().require("skill.read").map_err(hub_err)?;
    let (allowlist, catalog) = skill_governance(runtime, &working_directory, Some(&args.name))?;
    if !allowlist.contains(&args.name) {
        return Err(CoreError::Other(anyhow::anyhow!(
            "skill not on host/project allowlist"
        )));
    }
    if let Some(expected) = hub
        .skill_digests
        .lock()
        .ok()
        .and_then(|g| g.get(&args.name).cloned())
    {
        if catalog.digest(&args.name) != Some(expected.as_str()) {
            return Err(CoreError::Other(anyhow::anyhow!(
                "skill changed after discovery; review and reload"
            )));
        }
    }
    let instructions = catalog
        .activate(&ctx, &args.name, &allowlist)
        .map_err(hub_err)?;
    Ok(json!({
        "activated": args.name,
        "untrusted": true,
        "instructions": instructions,
        "note": "SKILL.md allowed-tools never grants capabilities"
    }))
}

pub(super) fn child_kernel_limits(parent: &RunContext) -> anycode_harness_core::types::Limits {
    let available = parent
        .budget()
        .snapshot()
        .map(|s| s.limit.saturating_sub(s.spent).saturating_sub(s.reserved))
        .unwrap_or(64);
    let reservation = available
        .saturating_div(4)
        .clamp(64, 32_768)
        .min(available.max(1));
    anycode_harness_core::types::Limits {
        reservation_per_hop: reservation,
        max_turns: 8,
        ..anycode_harness_core::types::Limits::default()
    }
}

fn consume_scoped_ticket(
    hub: &HarnessHub,
    ctx: &RunContext,
    ticket: Uuid,
) -> Result<Uuid, CoreError> {
    let device = ctx
        .scope()
        .device
        .ok_or_else(|| CoreError::Other(anyhow::anyhow!("computer device is not in scope")))?;
    hub.consume_ticket(ticket, ctx.id(), device)
        .map_err(hub_err)?;
    Ok(device)
}

fn enrolled_computer(
    hub: &HarnessHub,
    device: Uuid,
) -> Result<(Arc<ComputerBroker>, String), CoreError> {
    let slot = hub
        .computer
        .lock()
        .map_err(|_| CoreError::Other(anyhow::anyhow!("computer lock")))?;
    let enrolled = slot.as_ref().ok_or_else(|| {
        CoreError::Other(anyhow::anyhow!(
            "computer observe/act is fail-closed until a host-enrolled backend is attached"
        ))
    })?;
    if enrolled.device != device {
        return Err(CoreError::Other(anyhow::anyhow!(
            "enrolled computer device does not match scope"
        )));
    }
    Ok((enrolled.broker.clone(), enrolled.kind.clone()))
}

fn hub_err(err: anycode_harness_core::Error) -> CoreError {
    CoreError::Other(anyhow::anyhow!(err.to_string()))
}
