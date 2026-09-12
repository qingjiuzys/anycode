//! Deterministic local contract demo; no model connection or cloud authentication.
use anycode_harness_core::{
    budget::BudgetPool,
    events::PreviewBus,
    journal::MemoryJournal,
    kernel::{ControlQueue, Host, Kernel},
    types::*,
    Capabilities, Result, RunContext, Scope,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use uuid::Uuid;
struct Demo(AtomicUsize);
#[async_trait]
impl Host for Demo {
    fn tools(&self) -> Result<Vec<ToolSpec>> {
        Ok(vec![ToolSpec {
            name: "inspect_fixture".into(),
            description: "Read a fixed demo value, no filesystem".into(),
            input_schema: json!({"type":"object","additionalProperties":false}),
            capability: "fixture.read".into(),
            read_only: true,
        }])
    }
    async fn infer(&self, _: &RunContext, _: Vec<Value>, _: Vec<ToolSpec>) -> Result<Hop> {
        let calls = if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
            vec![Invocation {
                id: "fixture-1".into(),
                name: "inspect_fixture".into(),
                arguments: json!({}),
            }]
        } else {
            vec![]
        };
        Ok(Hop {
            assistant: json!({"role":"assistant","text":"scripted fixture response","calls":calls}),
            calls,
            usage: Some(Usage {
                input_tokens: 5,
                output_tokens: 2,
                ..Usage::default()
            }),
        })
    }
    async fn invoke_checked(&self, ctx: &RunContext, _: &Invocation) -> Result<ToolResult> {
        ctx.capabilities().require("fixture.read")?;
        Ok(ToolResult {
            value: json!({"fixture":true}),
            is_error: false,
        })
    }
    fn user_message(&self, text: &str) -> Result<Value> {
        Ok(json!({"role":"user","text":text}))
    }
    fn result_message(&self, call: &Invocation, result: &ToolResult) -> Result<Value> {
        Ok(json!({"role":"tool","id":call.id,"result":result.value}))
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let host = Demo(AtomicUsize::new(0));
    let ctx = RunContext::root(
        Scope {
            subject: Uuid::new_v4(),
            organization: None,
            tenant: None,
            project: Uuid::new_v4(),
            device: None,
        },
        Capabilities::new(["fixture.read".into()])?,
        BudgetPool::new(1000)?,
        Duration::from_secs(10),
    )?;
    let journal = MemoryJournal::default();
    let controls = ControlQueue::default();
    let kernel = Kernel {
        host: &host,
        journal: &journal,
        controls: &controls,
        previews: PreviewBus::default(),
        limits: Limits {
            reservation_per_hop: 50,
            ..Limits::default()
        },
    };
    kernel
        .run(&ctx, vec![host.user_message("inspect the fixture")?])
        .await?;
    for record in journal.records()? {
        println!("{} {}", record.sequence, record.event.kind);
    }
    println!(
        "DEMO ONLY — measured fixture tokens: {}",
        ctx.budget().snapshot()?.spent
    );
    Ok(())
}
