use anycode_harness_core::{
    budget::BudgetPool,
    events::PreviewBus,
    journal::{EventSink, MemoryJournal},
    kernel::{ControlQueue, Host, Kernel},
    types::*,
    Capabilities, Error, Result, RunContext, Scope,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
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
        Capabilities::new(["fs.read".into(), "fs.write".into()]).unwrap(),
        BudgetPool::new(1000).unwrap(),
        Duration::from_secs(5),
    )
    .unwrap()
}
fn call(id: &str, name: &str) -> Invocation {
    Invocation {
        id: id.into(),
        name: name.into(),
        arguments: json!({}),
    }
}
fn hop(calls: Vec<Invocation>) -> Hop {
    Hop {
        assistant: json!({"role":"assistant","calls":calls}),
        calls,
        usage: Some(Usage {
            input_tokens: 3,
            output_tokens: 2,
            ..Usage::default()
        }),
    }
}
struct Fake {
    responses: Mutex<VecDeque<Hop>>,
    invoked: AtomicUsize,
    uncertain: bool,
    deny_completion: bool,
    repair_completion: Mutex<bool>,
}
impl Fake {
    fn new(responses: Vec<Hop>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
            invoked: AtomicUsize::new(0),
            uncertain: false,
            deny_completion: false,
            repair_completion: Mutex::new(false),
        }
    }
}
#[async_trait]
impl Host for Fake {
    fn tools(&self) -> Result<Vec<ToolSpec>> {
        Ok(vec![
            ToolSpec {
                name: "read".into(),
                description: "read".into(),
                input_schema: json!({}),
                capability: "fs.read".into(),
                read_only: true,
            },
            ToolSpec {
                name: "write".into(),
                description: "write".into(),
                input_schema: json!({}),
                capability: "fs.write".into(),
                read_only: false,
            },
        ])
    }
    async fn infer(&self, _: &RunContext, _: Vec<Value>, _: Vec<ToolSpec>) -> Result<Hop> {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| Error::Host("fake exhausted".into()))
    }
    async fn invoke_checked(&self, _: &RunContext, _: &Invocation) -> Result<ToolResult> {
        self.invoked.fetch_add(1, Ordering::SeqCst);
        if self.uncertain {
            return Err(Error::Uncertain("network lost".into()));
        }
        Ok(ToolResult {
            value: json!({"ok":true}),
            is_error: false,
        })
    }
    fn user_message(&self, text: &str) -> Result<Value> {
        Ok(json!({"role":"user","text":text}))
    }
    fn result_message(&self, call: &Invocation, result: &ToolResult) -> Result<Value> {
        Ok(json!({"role":"tool","id":call.id,"is_error":result.is_error}))
    }
    async fn accept_completion(&self, _: &RunContext, _: &[Value]) -> Result<Option<String>> {
        if self.deny_completion {
            return Err(Error::Denied("missing evidence".into()));
        }
        let mut repair = self.repair_completion.lock().unwrap();
        if *repair {
            *repair = false;
            return Ok(Some("repair the deliverable".into()));
        }
        Ok(None)
    }
}
fn kernel<'a>(
    host: &'a dyn Host,
    journal: &'a dyn EventSink,
    queue: &'a ControlQueue,
) -> Kernel<'a> {
    Kernel {
        host,
        journal,
        previews: PreviewBus::default(),
        controls: queue,
        limits: Limits {
            reservation_per_hop: 50,
            ..Limits::default()
        },
    }
}
#[tokio::test]
async fn one_loop_commits_intent_before_tool_and_records_usage() {
    let host = Fake::new(vec![hop(vec![call("1", "read")]), hop(vec![])]);
    let journal = MemoryJournal::default();
    let queue = ControlQueue::default();
    let c = ctx();
    let messages = kernel(&host, &journal, &queue)
        .run(&c, vec![])
        .await
        .unwrap();
    assert_eq!(host.invoked.load(Ordering::SeqCst), 1);
    assert_eq!(messages.len(), 3);
    assert_eq!(c.budget().snapshot().unwrap().spent, 10);
    let records = journal.records().unwrap();
    let intent = records
        .iter()
        .position(|r| r.event.kind == "tool_intent")
        .unwrap();
    let end = records
        .iter()
        .position(|r| r.event.kind == "tool_end")
        .unwrap();
    assert!(intent < end);
    assert_eq!(records.last().unwrap().event.data["status"], "completed");
}
#[tokio::test]
async fn duplicate_calls_are_rejected_before_any_tool() {
    let host = Fake::new(vec![hop(vec![call("same", "read"), call("same", "read")])]);
    let j = MemoryJournal::default();
    let q = ControlQueue::default();
    assert!(kernel(&host, &j, &q).run(&ctx(), vec![]).await.is_err());
    assert_eq!(host.invoked.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn denied_capability_is_not_a_sandbox_bypass() {
    let host = Fake::new(vec![hop(vec![call("1", "write")]), hop(vec![])]);
    let j = MemoryJournal::default();
    let q = ControlQueue::default();
    let p = ctx();
    let c = p
        .child(
            &Capabilities::new(["fs.read".into()]).unwrap(),
            2,
            Duration::from_secs(2),
        )
        .unwrap();
    kernel(&host, &j, &q).run(&c, vec![]).await.unwrap();
    assert_eq!(host.invoked.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn uncertain_effect_never_gets_tool_end_or_success() {
    let mut host = Fake::new(vec![hop(vec![call("1", "write")])]);
    host.uncertain = true;
    let j = MemoryJournal::default();
    let q = ControlQueue::default();
    assert!(matches!(
        kernel(&host, &j, &q).run(&ctx(), vec![]).await,
        Err(Error::Uncertain(_))
    ));
    let r = j.records().unwrap();
    assert!(!r.iter().any(|r| r.event.kind == "tool_end"));
    assert_eq!(r.last().unwrap().event.data["status"], "uncertain");
}
#[tokio::test]
async fn completion_hook_can_block_fake_success() {
    let mut host = Fake::new(vec![hop(vec![])]);
    host.deny_completion = true;
    let j = MemoryJournal::default();
    let q = ControlQueue::default();
    assert!(kernel(&host, &j, &q).run(&ctx(), vec![]).await.is_err());
}
#[tokio::test]
async fn run_id_cannot_be_executed_twice() {
    let host = Fake::new(vec![hop(vec![]), hop(vec![])]);
    let j = MemoryJournal::default();
    let q = ControlQueue::default();
    let c = ctx();
    kernel(&host, &j, &q).run(&c, vec![]).await.unwrap();
    assert!(matches!(
        kernel(&host, &j, &q).run(&c, vec![]).await,
        Err(Error::Conflict(_))
    ));
}
#[tokio::test]
async fn completion_repair_queues_another_turn() {
    let mut host = Fake::new(vec![hop(vec![]), hop(vec![])]);
    *host.repair_completion.lock().unwrap() = true;
    let j = MemoryJournal::default();
    let q = ControlQueue::default();
    let messages = kernel(&host, &j, &q).run(&ctx(), vec![]).await.unwrap();
    assert_eq!(messages.len(), 3);
}

#[tokio::test]
async fn queued_follow_up_gets_another_turn() {
    let host = Fake::new(vec![hop(vec![]), hop(vec![])]);
    let j = MemoryJournal::default();
    let q = ControlQueue::default();
    q.follow_up("also check".into()).unwrap();
    let messages = kernel(&host, &j, &q).run(&ctx(), vec![]).await.unwrap();
    assert_eq!(messages.len(), 3);
}
#[test]
fn usage_total_records_cache_tokens_without_currency() {
    let usage = Usage {
        input_tokens: 1,
        output_tokens: 2,
        cache_creation_tokens: 4,
        cache_read_tokens: 8,
    };
    assert_eq!(usage.total(), 15);
}

#[test]
fn restored_unknown_usage_is_charged() {
    let pool = BudgetPool::from_checkpoint(anycode_harness_core::budget::BudgetSnapshot {
        limit: 100,
        spent: 20,
        reserved: 30,
        overdrawn: false,
    })
    .unwrap();
    assert_eq!(pool.snapshot().unwrap().spent, 50);
    assert_eq!(pool.snapshot().unwrap().reserved, 0);
}
