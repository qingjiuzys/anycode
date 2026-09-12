//! Durable DAG orchestration with conditional edges, barrier joins and human pauses.
//! This is NOT a LangGraph binary/API adapter and does not silently accept arbitrary
//! cyclic state graphs. Bounded iteration can be expressed as explicit versioned nodes.
use crate::checkpoint::CheckpointStore;
use anycode_harness_core::{budget::BudgetSnapshot, digest::digest, Error, Result, RunContext};
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Graph {
    pub version: u32,
    pub name: String,
    pub nodes: Vec<Node>,
    #[serde(default = "one")]
    pub max_parallel: usize,
}
fn one() -> usize {
    1
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    #[serde(default)]
    pub depends_on: Vec<Dependency>,
    #[serde(default)]
    pub join: Join,
    #[serde(default = "one")]
    pub max_attempts: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum NodeKind {
    Work {
        agent: String,
        prompt: String,
    },
    Gate {
        verifier: String,
    },
    Branch {
        source: String,
        pointer: String,
        equals: Value,
    },
    Human {
        question: String,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub node: String,
    #[serde(default)]
    pub on: On,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum On {
    #[default]
    Completed,
    True,
    False,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Join {
    #[default]
    All,
    Any,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Pending,
    Running,
    Completed,
    Skipped,
    Partial,
    Failed,
    Waiting,
    Uncertain,
    Cancelled,
}
impl Status {
    fn terminal(self) -> bool {
        !matches!(self, Self::Pending | Self::Running | Self::Waiting)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NodeState {
    pub status: Status,
    pub attempts: usize,
    pub output: Value,
    pub error: Option<String>,
}
impl Default for NodeState {
    fn default() -> Self {
        Self {
            status: Status::Pending,
            attempts: 0,
            output: Value::Null,
            error: None,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Checkpoint {
    pub version: u32,
    pub run_id: Uuid,
    pub definition_digest: String,
    pub scope_digest: String,
    pub revision: u64,
    pub nodes: BTreeMap<String, NodeState>,
    pub budget: BudgetSnapshot,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphStatus {
    Completed,
    Waiting,
    Partial,
    Failed,
    Uncertain,
    Cancelled,
}
#[derive(Clone, Debug)]
pub struct GraphResult {
    pub status: GraphStatus,
    pub checkpoint: Checkpoint,
}
#[derive(Clone, Debug)]
pub enum NodeOutput {
    Completed(Value),
    Partial { output: Value, remaining: String },
}
#[derive(Clone, Debug)]
pub struct Verification {
    pub passed: bool,
    pub artifact_digest: String,
    pub report: Value,
}

#[async_trait]
pub trait NodeExecutor: Send + Sync {
    /// Run via the same Harness/AgentRuntime used by ordinary chat. Never introduce
    /// another provider loop here. Inputs contain only declared predecessors.
    async fn execute(
        &self,
        ctx: &RunContext,
        node: &Node,
        inputs: BTreeMap<String, Value>,
    ) -> Result<NodeOutput>;
    /// Separate trusted verification path. A Work node saying "passed" cannot pass a gate.
    async fn verify(
        &self,
        ctx: &RunContext,
        verifier: &str,
        inputs: BTreeMap<String, Value>,
    ) -> Result<Verification>;
    /// Only trusted host code can authorize retry. Graph/skill authors cannot upgrade it.
    fn safe_to_retry(&self, _node: &Node) -> bool {
        false
    }
    /// Same key serializes conflicting resources. Default is fully exclusive;
    /// isolated worktrees/read-only backends may return independent resource keys.
    fn concurrency_key(&self, _node: &Node) -> String {
        "exclusive".into()
    }
}
impl Graph {
    pub fn validate(&self) -> Result<()> {
        if self.version != 1
            || self.name.trim().is_empty()
            || self.name.len() > 128
            || self.nodes.is_empty()
            || self.nodes.len() > 512
            || !(1..=32).contains(&self.max_parallel)
        {
            return Err(Error::Invalid("graph version/name/size/concurrency".into()));
        }
        let mut ids = BTreeSet::new();
        for node in &self.nodes {
            if node.id.is_empty()
                || node.id.len() > 64
                || !node
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                || !ids.insert(node.id.clone())
                || !(1..=3).contains(&node.max_attempts)
            {
                return Err(Error::Invalid("node ID/attempts/duplicate".into()));
            }
            match &node.kind {
                NodeKind::Work { agent, prompt }
                    if agent.is_empty()
                        || agent.len() > 128
                        || prompt.is_empty()
                        || prompt.len() > 128 * 1024 =>
                {
                    return Err(Error::Invalid("work node bounds".into()))
                }
                NodeKind::Human { question } if question.is_empty() || question.len() > 8192 => {
                    return Err(Error::Invalid("human question bounds".into()))
                }
                NodeKind::Gate { verifier } if verifier.is_empty() || verifier.len() > 128 => {
                    return Err(Error::Invalid("empty verifier".into()))
                }
                _ => {}
            }
        }
        for node in &self.nodes {
            let mut predecessors: BTreeSet<String> = BTreeSet::new();
            for edge in &node.depends_on {
                if !ids.contains(&edge.node)
                    || edge.node == node.id
                    || !predecessors.insert(edge.node.clone())
                {
                    return Err(Error::Invalid("unknown/self/duplicate dependency".into()));
                }
                if !matches!(edge.on, On::Completed) {
                    let source = self
                        .nodes
                        .iter()
                        .find(|n| n.id == edge.node)
                        .ok_or_else(|| Error::Invalid("source".into()))?;
                    if !matches!(&source.kind, NodeKind::Branch { .. }) {
                        return Err(Error::Invalid(
                            "true/false edges require branch source".into(),
                        ));
                    }
                }
            }
            if let NodeKind::Branch {
                source, pointer, ..
            } = &node.kind
            {
                if !predecessors.contains(source)
                    || (!pointer.is_empty() && !pointer.starts_with('/'))
                {
                    return Err(Error::Invalid(
                        "branch source must be a direct dependency; pointer must be JSON Pointer"
                            .into(),
                    ));
                }
            }
        }
        let mut completed = BTreeSet::new();
        loop {
            let ready: Vec<_> = self
                .nodes
                .iter()
                .filter(|n| {
                    !completed.contains(&n.id)
                        && n.depends_on.iter().all(|d| completed.contains(&d.node))
                })
                .map(|n| n.id.clone())
                .collect();
            if ready.is_empty() {
                break;
            }
            completed.extend(ready);
        }
        if completed.len() != self.nodes.len() {
            return Err(Error::Unsupported(
                "cyclic graph; use an explicit bounded unrolling, not silent infinite loops".into(),
            ));
        }
        Ok(())
    }
    pub fn fingerprint(&self) -> Result<String> {
        self.validate()?;
        digest(self)
    }
}
impl Checkpoint {
    pub fn fresh(graph: &Graph, ctx: &RunContext) -> Result<Self> {
        Self::fresh_with_run(graph, ctx, Uuid::new_v4())
    }

    pub fn fresh_with_run(graph: &Graph, ctx: &RunContext, run_id: Uuid) -> Result<Self> {
        if run_id.is_nil() {
            return Err(Error::Invalid("graph run_id".into()));
        }
        Ok(Self {
            version: 1,
            run_id,
            definition_digest: graph.fingerprint()?,
            scope_digest: ctx.scope().binding()?,
            revision: 0,
            nodes: graph
                .nodes
                .iter()
                .map(|n| (n.id.clone(), NodeState::default()))
                .collect(),
            budget: ctx.budget().snapshot()?,
        })
    }
    pub fn validate(&self, graph: &Graph, ctx: &RunContext, expected_run: Uuid) -> Result<()> {
        if self.version != 1
            || self.run_id != expected_run
            || self.definition_digest != graph.fingerprint()?
            || self.scope_digest != ctx.scope().binding()?
            || self.nodes.keys().cloned().collect::<BTreeSet<_>>()
                != graph.nodes.iter().map(|n| n.id.clone()).collect()
        {
            return Err(Error::Conflict(
                "checkpoint run/definition/tenant/project mismatch".into(),
            ));
        }
        let live = ctx.budget().snapshot()?;
        if live.spent < self.budget.spent.saturating_add(self.budget.reserved)
            || live.limit > self.budget.limit
        {
            return Err(Error::Denied(
                "resume must carry forward the budget ledger, not reset/increase it".into(),
            ));
        }
        for node in &graph.nodes {
            if self.nodes[&node.id].attempts > node.max_attempts {
                return Err(Error::Invalid("checkpoint attempt count".into()));
            }
        }
        Ok(())
    }
}
pub struct GraphRunner<'a> {
    pub executor: &'a dyn NodeExecutor,
    pub store: &'a dyn CheckpointStore,
}
impl GraphRunner<'_> {
    fn save(&self, ctx: &RunContext, cp: &mut Checkpoint) -> Result<()> {
        cp.revision = cp
            .revision
            .checked_add(1)
            .ok_or_else(|| Error::Conflict("revision overflow".into()))?;
        cp.budget = ctx.budget().snapshot()?;
        self.store.save(&serde_json::to_value(cp)?)
    }
    /// Start is explicit and rejects an occupied store; resume is a different API.
    pub async fn start(&self, graph: &Graph, ctx: &RunContext) -> Result<GraphResult> {
        self.start_with_run(graph, ctx, Uuid::new_v4()).await
    }

    pub async fn start_with_run(
        &self,
        graph: &Graph,
        ctx: &RunContext,
        run_id: Uuid,
    ) -> Result<GraphResult> {
        ctx.check()?;
        let _lease = self.store.lease()?;
        if self.store.load()?.is_some() {
            return Err(Error::Conflict(
                "store already contains a run; explicit resume required".into(),
            ));
        }
        let mut cp = Checkpoint::fresh_with_run(graph, ctx, run_id)?;
        self.save(ctx, &mut cp)?;
        self.drive(graph, ctx, cp).await
    }
    pub async fn resume(
        &self,
        graph: &Graph,
        ctx: &RunContext,
        expected_run: Uuid,
    ) -> Result<GraphResult> {
        ctx.check()?;
        let _lease = self.store.lease()?;
        let raw = self
            .store
            .load()?
            .ok_or_else(|| Error::Invalid("no checkpoint".into()))?;
        let mut cp: Checkpoint = serde_json::from_value(raw)?;
        cp.validate(graph, ctx, expected_run)?;
        if cp.nodes.values().any(|n| n.status == Status::Running) {
            for node in cp
                .nodes
                .values_mut()
                .filter(|n| n.status == Status::Running)
            {
                node.status = Status::Uncertain;
                node.error = Some(
                    "interrupted execution; reconcile side effects AND provider usage before retry"
                        .into(),
                );
            }
            self.save(ctx, &mut cp)?;
            return Ok(GraphResult {
                status: GraphStatus::Uncertain,
                checkpoint: cp,
            });
        }
        self.drive(graph, ctx, cp).await
    }
    /// HOST ONLY, after fresh account + ACL + CSRF checks. This records a workflow
    /// decision, not a blanket capability grant. Stale approvals fail revision checks.
    pub fn resolve_human(
        &self,
        graph: &Graph,
        ctx: &RunContext,
        expected_run: Uuid,
        revision: u64,
        node_id: &str,
        approved: bool,
    ) -> Result<Checkpoint> {
        ctx.check()?;
        let _lease = self.store.lease()?;
        let mut cp: Checkpoint = serde_json::from_value(
            self.store
                .load()?
                .ok_or_else(|| Error::Invalid("no checkpoint".into()))?,
        )?;
        cp.validate(graph, ctx, expected_run)?;
        if cp.revision != revision
            || !graph
                .nodes
                .iter()
                .any(|n| n.id == node_id && matches!(&n.kind, NodeKind::Human { .. }))
        {
            return Err(Error::Conflict("stale/nonhuman approval".into()));
        }
        let node = cp
            .nodes
            .get_mut(node_id)
            .ok_or_else(|| Error::Invalid("unknown node".into()))?;
        if node.status != Status::Waiting {
            return Err(Error::Conflict("node is not awaiting a decision".into()));
        }
        node.status = if approved {
            Status::Completed
        } else {
            Status::Failed
        };
        node.output = json!({"approved": approved, "approver": ctx.scope().subject});
        self.save(ctx, &mut cp)?;
        Ok(cp)
    }
    async fn drive(
        &self,
        graph: &Graph,
        ctx: &RunContext,
        mut cp: Checkpoint,
    ) -> Result<GraphResult> {
        loop {
            if ctx.check().is_err() {
                for state in cp
                    .nodes
                    .values_mut()
                    .filter(|s| matches!(s.status, Status::Pending | Status::Waiting))
                {
                    state.status = Status::Cancelled;
                }
                self.save(ctx, &mut cp)?;
                let status = if cp
                    .nodes
                    .values()
                    .any(|s| matches!(s.status, Status::Uncertain | Status::Running))
                {
                    GraphStatus::Uncertain
                } else {
                    GraphStatus::Cancelled
                };
                return Ok(GraphResult {
                    status,
                    checkpoint: cp,
                });
            }
            for (status, result) in [
                (Status::Uncertain, GraphStatus::Uncertain),
                (Status::Failed, GraphStatus::Failed),
                (Status::Partial, GraphStatus::Partial),
                (Status::Cancelled, GraphStatus::Cancelled),
            ] {
                if cp.nodes.values().any(|n| n.status == status) {
                    return Ok(GraphResult {
                        status: result,
                        checkpoint: cp,
                    });
                }
            }
            if cp
                .nodes
                .values()
                .all(|n| matches!(n.status, Status::Completed | Status::Skipped))
            {
                return Ok(GraphResult {
                    status: GraphStatus::Completed,
                    checkpoint: cp,
                });
            }
            if cp.nodes.values().any(|n| n.status == Status::Waiting) {
                return Ok(GraphResult {
                    status: GraphStatus::Waiting,
                    checkpoint: cp,
                });
            }
            let mut eligible = vec![];
            let mut changed = false;
            // Declaration order is deterministic. Outputs are namespaced by node,
            // so parallel siblings never race to overwrite a global mutable state map.
            for node in &graph.nodes {
                if cp.nodes[&node.id].status != Status::Pending {
                    continue;
                }
                if !node
                    .depends_on
                    .iter()
                    .all(|d| cp.nodes[&d.node].status.terminal())
                {
                    continue;
                }
                let matched = node
                    .depends_on
                    .iter()
                    .filter(|d| {
                        let source = &cp.nodes[&d.node];
                        source.status == Status::Completed
                            && match d.on {
                                On::Completed => true,
                                On::True => source.output == Value::Bool(true),
                                On::False => source.output == Value::Bool(false),
                            }
                    })
                    .count();
                let enabled = node.depends_on.is_empty()
                    || match node.join {
                        Join::All => matched == node.depends_on.len(),
                        Join::Any => matched > 0,
                    };
                if !enabled {
                    cp.nodes
                        .get_mut(&node.id)
                        .ok_or_else(|| Error::Invalid("node".into()))?
                        .status = Status::Skipped;
                    changed = true;
                    continue;
                }
                match &node.kind {
                    NodeKind::Human { question } => {
                        let state = cp
                            .nodes
                            .get_mut(&node.id)
                            .ok_or_else(|| Error::Invalid("node".into()))?;
                        state.status = Status::Waiting;
                        state.output = json!({"question": question});
                        changed = true;
                    }
                    NodeKind::Branch {
                        source,
                        pointer,
                        equals,
                    } => {
                        let value = cp.nodes[source]
                            .output
                            .pointer(pointer)
                            .map(|v| v == equals);
                        let state = cp
                            .nodes
                            .get_mut(&node.id)
                            .ok_or_else(|| Error::Invalid("node".into()))?;
                        match value {
                            Some(value) => {
                                state.status = Status::Completed;
                                state.output = Value::Bool(value);
                            }
                            None => {
                                state.status = Status::Failed;
                                state.error = Some("branch_json_pointer_missing".into());
                            }
                        }
                        changed = true;
                    }
                    _ => eligible.push(node),
                }
            }
            if changed {
                self.save(ctx, &mut cp)?;
            }
            if cp
                .nodes
                .values()
                .any(|n| matches!(n.status, Status::Waiting | Status::Failed))
            {
                continue;
            }
            let mut resources = BTreeSet::new();
            let chosen: Vec<_> = eligible
                .into_iter()
                .filter(|n| resources.insert(self.executor.concurrency_key(n)))
                .take(graph.max_parallel)
                .collect();
            if chosen.is_empty() {
                if changed {
                    continue;
                }
                return Err(Error::Conflict(
                    "nonterminal graph has no schedulable nodes".into(),
                ));
            }
            let mut pending = FuturesUnordered::new();
            for node in &chosen {
                let state = cp
                    .nodes
                    .get_mut(&node.id)
                    .ok_or_else(|| Error::Invalid("node".into()))?;
                if state.attempts >= node.max_attempts {
                    return Err(Error::Conflict("retry budget exhausted".into()));
                }
                state.status = Status::Running;
                state.attempts += 1;
            }
            self.save(ctx, &mut cp)?; // all intents durably RUNNING before any execution
            for node in chosen {
                let inputs = node
                    .depends_on
                    .iter()
                    .filter(|d| cp.nodes[&d.node].status == Status::Completed)
                    .map(|d| (d.node.clone(), cp.nodes[&d.node].output.clone()))
                    .collect();
                pending.push(async move {
                    let result = tokio::select! {
                        _ = ctx.cancelled() => Err(Error::Uncertain("node interrupted; effects may have happened".into())),
                        result = self.execute_node(ctx, node, inputs) => result,
                    };
                    (node, result)
                });
            }
            while let Some((node, result)) = pending.next().await {
                let state = cp
                    .nodes
                    .get_mut(&node.id)
                    .ok_or_else(|| Error::Invalid("node".into()))?;
                match result {
                    Ok(NodeOutput::Completed(output)) => {
                        state.status = Status::Completed;
                        state.output = output;
                        state.error = None;
                    }
                    Ok(NodeOutput::Partial { output, remaining }) => {
                        state.status = Status::Partial;
                        state.output = output;
                        state.error = Some(remaining);
                    }
                    Err(Error::Uncertain(_)) | Err(Error::Cancelled) => {
                        state.status = Status::Uncertain;
                        state.error = Some("reconciliation_required".into());
                    }
                    Err(_)
                        if self.executor.safe_to_retry(node)
                            && state.attempts < node.max_attempts =>
                    {
                        state.status = Status::Pending;
                        state.error = Some("safe_retry_scheduled".into());
                    }
                    Err(err) => {
                        state.status = Status::Failed;
                        state.error = Some(err.to_string());
                    }
                }
                // Store failure aborts the run with remaining RUNNING intents, not fake success.
                self.save(ctx, &mut cp)?;
            }
        }
    }
    async fn execute_node(
        &self,
        ctx: &RunContext,
        node: &Node,
        inputs: BTreeMap<String, Value>,
    ) -> Result<NodeOutput> {
        let result = match &node.kind {
            NodeKind::Gate { verifier } => {
                let evidence = self.executor.verify(ctx, verifier, inputs).await?;
                if !evidence.passed
                    || evidence.artifact_digest.len() != 64
                    || !evidence
                        .artifact_digest
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit())
                {
                    return Err(Error::Denied("gate missing valid evidence".into()));
                }
                NodeOutput::Completed(
                    json!({"verified": true, "verifier": verifier, "artifact_digest": evidence.artifact_digest, "report": evidence.report}),
                )
            }
            NodeKind::Work { .. } => self.executor.execute(ctx, node, inputs).await?,
            _ => return Err(Error::Invalid("internal node dispatch".into())),
        };
        let output = match &result {
            NodeOutput::Completed(v) | NodeOutput::Partial { output: v, .. } => v,
        };
        if serde_json::to_vec(output)?.len() > 256 * 1024 {
            return Err(Error::Invalid(
                "node output too large; persist artifact references".into(),
            ));
        }
        Ok(result)
    }
}
