use anycode_harness_core::{budget::BudgetPool, Capabilities, Error, Result, RunContext, Scope};
use anycode_harness_extensions::{
    checkpoint::{CheckpointStore, MemoryCheckpoint},
    graph::*,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use uuid::Uuid;
fn ctx() -> RunContext {
    RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new(["agent.spawn".into()]).unwrap(),
        BudgetPool::new(1000).unwrap(),
        Duration::from_secs(10),
    )
    .unwrap()
}
fn work(id: &str, deps: &[&str]) -> Node {
    Node {
        id: id.into(),
        kind: NodeKind::Work {
            agent: "worker".into(),
            prompt: "task".into(),
        },
        depends_on: deps
            .iter()
            .map(|id| Dependency {
                node: (*id).into(),
                on: On::Completed,
            })
            .collect(),
        join: Join::All,
        max_attempts: 1,
    }
}
fn graph(nodes: Vec<Node>) -> Graph {
    Graph {
        version: 1,
        name: "test".into(),
        nodes,
        max_parallel: 4,
    }
}
struct Ex {
    count: AtomicUsize,
    partial: bool,
    gate: bool,
}
impl Default for Ex {
    fn default() -> Self {
        Self {
            count: AtomicUsize::new(0),
            partial: false,
            gate: true,
        }
    }
}
#[async_trait]
impl NodeExecutor for Ex {
    async fn execute(
        &self,
        _: &RunContext,
        _: &Node,
        _: BTreeMap<String, Value>,
    ) -> Result<NodeOutput> {
        self.count.fetch_add(1, Ordering::SeqCst);
        if self.partial {
            Ok(NodeOutput::Partial {
                output: json!({}),
                remaining: "not tested".into(),
            })
        } else {
            Ok(NodeOutput::Completed(json!({"ok":true})))
        }
    }
    async fn verify(
        &self,
        _: &RunContext,
        _: &str,
        _: BTreeMap<String, Value>,
    ) -> Result<Verification> {
        Ok(Verification {
            passed: self.gate,
            artifact_digest: "a".repeat(64),
            report: json!({"exit_code":0}),
        })
    }
}
#[test]
fn cycles_and_unknown_dependencies_are_rejected() {
    assert!(graph(vec![work("a", &["b"]), work("b", &["a"])])
        .validate()
        .is_err());
    assert!(graph(vec![work("a", &["missing"])]).validate().is_err());
}
#[tokio::test]
async fn linear_runs_and_completed_resume_does_not_repeat() {
    let c = ctx();
    let g = graph(vec![work("a", &[]), work("b", &["a"])]);
    let s = MemoryCheckpoint::default();
    let ex = Ex::default();
    let r = GraphRunner {
        executor: &ex,
        store: &s,
    };
    let first = r.start(&g, &c).await.unwrap();
    assert_eq!(first.status, GraphStatus::Completed);
    r.resume(&g, &c, first.checkpoint.run_id).await.unwrap();
    assert_eq!(ex.count.load(Ordering::SeqCst), 2);
    assert!(r.start(&g, &c).await.is_err());
}
#[tokio::test]
async fn partial_blocks_downstream() {
    let c = ctx();
    let g = graph(vec![work("a", &[]), work("b", &["a"])]);
    let s = MemoryCheckpoint::default();
    let ex = Ex {
        partial: true,
        ..Ex::default()
    };
    let out = GraphRunner {
        executor: &ex,
        store: &s,
    }
    .start(&g, &c)
    .await
    .unwrap();
    assert_eq!(out.status, GraphStatus::Partial);
    assert_eq!(ex.count.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn gate_failure_does_not_pass() {
    let c = ctx();
    let mut gate = work("verify", &["a"]);
    gate.kind = NodeKind::Gate {
        verifier: "tests".into(),
    };
    let g = graph(vec![work("a", &[]), gate]);
    let s = MemoryCheckpoint::default();
    let ex = Ex {
        gate: false,
        ..Ex::default()
    };
    assert_eq!(
        GraphRunner {
            executor: &ex,
            store: &s
        }
        .start(&g, &c)
        .await
        .unwrap()
        .status,
        GraphStatus::Failed
    );
}
#[tokio::test]
async fn human_pause_needs_matching_revision() {
    let c = ctx();
    let mut human = work("approve", &[]);
    human.kind = NodeKind::Human {
        question: "Proceed?".into(),
    };
    let g = graph(vec![human, work("a", &["approve"])]);
    let s = MemoryCheckpoint::default();
    let ex = Ex::default();
    let r = GraphRunner {
        executor: &ex,
        store: &s,
    };
    let wait = r.start(&g, &c).await.unwrap();
    assert_eq!(wait.status, GraphStatus::Waiting);
    let id = wait.checkpoint.run_id;
    assert!(r.resolve_human(&g, &c, id, 0, "approve", true).is_err());
    r.resolve_human(&g, &c, id, wait.checkpoint.revision, "approve", true)
        .unwrap();
    assert_eq!(
        r.resume(&g, &c, id).await.unwrap().status,
        GraphStatus::Completed
    );
}
#[tokio::test]
async fn mismatched_definition_or_scope_cannot_resume() {
    let c = ctx();
    let g = graph(vec![work("a", &[])]);
    let s = MemoryCheckpoint::default();
    let ex = Ex::default();
    let r = GraphRunner {
        executor: &ex,
        store: &s,
    };
    let out = r.start(&g, &c).await.unwrap();
    let id = out.checkpoint.run_id;
    assert!(r.resume(&g, &ctx(), id).await.is_err());
    let mut changed = g;
    changed.nodes[0].kind = NodeKind::Work {
        agent: "worker".into(),
        prompt: "different".into(),
    };
    assert!(r.resume(&changed, &c, id).await.is_err());
}
#[tokio::test]
async fn running_checkpoint_becomes_uncertain_not_replayed() {
    let c = ctx();
    let g = graph(vec![work("a", &[])]);
    let s = MemoryCheckpoint::default();
    let ex = Ex::default();
    let mut cp = Checkpoint::fresh(&g, &c).unwrap();
    cp.nodes.get_mut("a").unwrap().status = Status::Running;
    cp.nodes.get_mut("a").unwrap().attempts = 1;
    s.save(&serde_json::to_value(&cp).unwrap()).unwrap();
    let out = GraphRunner {
        executor: &ex,
        store: &s,
    }
    .resume(&g, &c, cp.run_id)
    .await
    .unwrap();
    assert_eq!(out.status, GraphStatus::Uncertain);
    assert_eq!(ex.count.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn branch_skips_false_arm_and_any_join_waits_for_barrier() {
    let c = ctx();
    let a = work("a", &[]);
    let mut branch = work("branch", &["a"]);
    branch.kind = NodeKind::Branch {
        source: "a".into(),
        pointer: "/ok".into(),
        equals: json!(true),
    };
    let mut yes = work("yes", &["branch"]);
    yes.depends_on[0].on = On::True;
    let mut no = work("no", &["branch"]);
    no.depends_on[0].on = On::False;
    let mut join = work("join", &["yes", "no"]);
    join.join = Join::Any;
    let g = graph(vec![a, branch, yes, no, join]);
    let ex = Ex::default();
    let s = MemoryCheckpoint::default();
    let out = GraphRunner {
        executor: &ex,
        store: &s,
    }
    .start(&g, &c)
    .await
    .unwrap();
    assert_eq!(out.status, GraphStatus::Completed);
    assert_eq!(out.checkpoint.nodes["no"].status, Status::Skipped);
    assert_eq!(ex.count.load(Ordering::SeqCst), 3);
}
#[test]
fn checkpoint_cannot_reset_budget() {
    let c = ctx();
    let g = graph(vec![work("a", &[])]);
    let mut cp = Checkpoint::fresh(&g, &c).unwrap();
    cp.budget.spent = 10;
    assert!(matches!(
        cp.validate(&g, &c, cp.run_id),
        Err(Error::Denied(_))
    ));
}
