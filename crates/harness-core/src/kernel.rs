//! One host-neutral loop. Tools are sequential in v1 for conservative side-effect
//! ordering; DAG/subagent concurrency lives outside this module. This is a deliberate
//! policy choice, not a claim about the current upstream Pi default.
use crate::{
    events::{Event, PreviewBus},
    journal::EventSink,
    types::*,
    Error, Result, RunContext,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashSet, VecDeque},
    sync::Mutex,
};

#[async_trait]
pub trait Host: Send + Sync {
    fn tools(&self) -> Result<Vec<ToolSpec>>;
    /// Preserve provider metadata/tool-call IDs. Never convert a partial stream to Hop.
    async fn infer(
        &self,
        ctx: &RunContext,
        messages: Vec<ProviderMessage>,
        tools: Vec<ToolSpec>,
    ) -> Result<Hop>;
    /// Host MUST validate full arguments and recheck its live authorization/sandbox
    /// before any side effect. Calling a bare legacy Tool::execute is not sufficient.
    async fn invoke_checked(&self, ctx: &RunContext, call: &Invocation) -> Result<ToolResult>;
    fn user_message(&self, text: &str) -> Result<ProviderMessage>;
    fn result_message(&self, call: &Invocation, result: &ToolResult) -> Result<ProviderMessage>;
    /// Compaction/resource loading belongs here, not inside the LLM provider transport.
    /// Return a request-only projection; persisted transcript is not overwritten.
    async fn transform_context(
        &self,
        _ctx: &RunContext,
        messages: &[ProviderMessage],
    ) -> Result<Vec<ProviderMessage>> {
        Ok(messages.to_vec())
    }
    /// Lifecycle hook for trusted policies such as completion evidence. Not a model flag.
    /// `Ok(None)` accepts completion. `Ok(Some(repair))` queues another turn.
    async fn accept_completion(
        &self,
        _ctx: &RunContext,
        _history: &[ProviderMessage],
    ) -> Result<Option<String>> {
        Ok(None)
    }
}
#[derive(Default)]
pub struct ControlQueue {
    steering: Mutex<VecDeque<String>>,
    follow_up: Mutex<VecDeque<String>>,
}
impl ControlQueue {
    fn push(queue: &Mutex<VecDeque<String>>, text: String) -> Result<()> {
        if text.is_empty() || text.len() > 8192 {
            return Err(Error::Invalid("queued message bounds".into()));
        }
        let mut queue = queue
            .lock()
            .map_err(|_| Error::Host("control queue lock".into()))?;
        if queue.len() >= 32 {
            return Err(Error::Capacity);
        }
        queue.push_back(text);
        Ok(())
    }
    pub fn steer(&self, text: String) -> Result<()> {
        Self::push(&self.steering, text)
    }
    pub fn follow_up(&self, text: String) -> Result<()> {
        Self::push(&self.follow_up, text)
    }
    fn drain(queue: &Mutex<VecDeque<String>>) -> Result<Vec<String>> {
        Ok(queue
            .lock()
            .map_err(|_| Error::Host("control queue lock".into()))?
            .drain(..)
            .collect())
    }
}
pub struct Kernel<'a> {
    pub host: &'a dyn Host,
    pub journal: &'a dyn EventSink,
    pub previews: PreviewBus,
    pub controls: &'a ControlQueue,
    pub limits: Limits,
}
impl Kernel<'_> {
    fn emit(&self, ctx: &RunContext, kind: &str, data: Value) -> Result<()> {
        let record = self.journal.append(Event::new(ctx, kind, data)?)?;
        // Raw transcripts may contain provider-private reasoning or sensitive outputs.
        // UI receives metadata only; authenticated adapters may publish filtered text.
        self.previews
            .publish(json!({"sequence":record.sequence,"event":{
                "version":record.event.version,"run_id":record.event.run_id,
                "root_id":record.event.root_id,"parent_run_id":record.event.parent_run_id,
                "scope_digest":record.event.scope_digest,"kind":record.event.kind
            }}));
        Ok(())
    }
    fn append(&self, ctx: &RunContext, history: &mut Vec<Value>, message: Value) -> Result<()> {
        self.emit(ctx, "message_committed", json!({"message": message}))?;
        history.push(message);
        Ok(())
    }
    pub async fn run(
        &self,
        ctx: &RunContext,
        mut history: Vec<ProviderMessage>,
    ) -> Result<Vec<ProviderMessage>> {
        ctx.check()?;
        if self.limits.max_turns == 0
            || self.limits.max_tools_per_turn == 0
            || self.limits.reservation_per_hop == 0
        {
            return Err(Error::Invalid("invalid loop limits".into()));
        }
        if serde_json::to_vec(&history)?.len() > self.limits.max_context_bytes {
            return Err(Error::Invalid("initial history exceeds byte limit".into()));
        }
        self.emit(
            ctx,
            "run_start",
            json!({"depth": ctx.depth(), "initial_history": history}),
        )?;
        let outcome = self.run_inner(ctx, &mut history).await;
        let status = match &outcome {
            Ok(()) => "completed",
            Err(Error::Cancelled) => "cancelled",
            Err(Error::Uncertain(_)) => "uncertain",
            Err(_) => "failed",
        };
        // Never put arbitrary host error bodies (possibly credentials) in the event feed.
        self.emit(
            ctx,
            "run_end",
            json!({"status": status, "budget": ctx.budget().snapshot()?}),
        )?;
        outcome?;
        Ok(history)
    }
    async fn run_inner(&self, ctx: &RunContext, history: &mut Vec<ProviderMessage>) -> Result<()> {
        let mut catalog = BTreeMap::new();
        for tool in self.host.tools()? {
            if tool.name.is_empty() || catalog.len() >= 1024 || catalog.contains_key(&tool.name) {
                return Err(Error::Invalid("invalid/duplicate tool registry".into()));
            }
            catalog.insert(tool.name.clone(), tool);
        }
        let mut seen_calls = HashSet::new();
        for turn in 1..=self.limits.max_turns {
            ctx.check()?;
            for text in ControlQueue::drain(&self.controls.steering)? {
                self.append(ctx, history, self.host.user_message(&text)?)?;
            }
            let request = tokio::select! {
                _ = ctx.cancelled() => return Err(Error::Cancelled),
                result = self.host.transform_context(ctx, history) => result?,
            };
            if serde_json::to_vec(&request)?.len() > self.limits.max_context_bytes {
                return Err(Error::Invalid(
                    "context bytes exceeded; compaction required".into(),
                ));
            }
            let tools = catalog
                .values()
                .filter(|t| ctx.capabilities().allows(&t.capability))
                .cloned()
                .collect();
            let mut reservation = ctx.budget().reserve(self.limits.reservation_per_hop)?;
            self.emit(ctx, "turn_start", json!({"turn": turn}))?;
            ctx.check()?;
            reservation.mark_sent();
            let hop = tokio::select! {
                _ = ctx.cancelled() => return Err(Error::Cancelled),
                result = self.host.infer(ctx, request, tools) => result?,
            };
            if let Some(usage) = hop.usage {
                self.emit(
                    ctx,
                    "usage",
                    json!({"turn": turn, "usage": usage, "measured": true}),
                )?;
                reservation.settle(usage.total())?;
            } else {
                self.emit(ctx, "usage", json!({"turn": turn, "reserved_maximum": self.limits.reservation_per_hop, "measured": false}))?;
                drop(reservation);
            }
            if hop.calls.len() > self.limits.max_tools_per_turn {
                return Err(Error::Invalid("too many tool calls".into()));
            }
            if serde_json::to_vec(&hop.assistant)?.len() > self.limits.max_context_bytes {
                return Err(Error::Invalid("assistant message too large".into()));
            }
            for call in &hop.calls {
                if call.id.is_empty() || call.id.len() > 256 || !seen_calls.insert(call.id.clone())
                {
                    return Err(Error::Invalid("duplicate/invalid invocation ID".into()));
                }
            }
            self.append(ctx, history, hop.assistant)?;
            let no_calls = hop.calls.is_empty();
            let mut steering_pending = ControlQueue::drain(&self.controls.steering)?;
            for call in &hop.calls {
                ctx.check()?;
                let result = if !steering_pending.is_empty() {
                    ToolResult {
                        value: json!({"error": "skipped_due_to_steering"}),
                        is_error: true,
                    }
                } else if let Some(spec) = catalog.get(&call.name) {
                    if !ctx.capabilities().allows(&spec.capability) {
                        ToolResult {
                            value: json!({"error": "capability_denied"}),
                            is_error: true,
                        }
                    } else if serde_json::to_vec(&call.arguments)?.len()
                        > self.limits.max_arguments_bytes
                    {
                        ToolResult {
                            value: json!({"error": "arguments_too_large"}),
                            is_error: true,
                        }
                    } else {
                        self.emit(ctx, "tool_intent", json!({"invocation_id": call.id, "tool": call.name,
                            "arguments_digest": crate::digest::digest(&call.arguments)?, "read_only": spec.read_only}))?;
                        // On cancellation, a mutating tool may already have acted. Its
                        // unresolved intent remains in the journal for reconciliation.
                        let invoked = tokio::select! {
                            _ = ctx.cancelled() => {
                                return Err(if spec.read_only { Error::Cancelled }
                                    else { Error::Uncertain(format!("tool {} interrupted", call.id)) });
                            }
                            result = self.host.invoke_checked(ctx, call) => result,
                        };
                        let mut output = match invoked {
                            Ok(value) => value,
                            Err(Error::Uncertain(reason)) => return Err(Error::Uncertain(reason)),
                            Err(Error::Cancelled) if !spec.read_only => {
                                return Err(Error::Uncertain(call.id.clone()))
                            }
                            Err(Error::Denied(_))
                            | Err(Error::Invalid(_))
                            | Err(Error::Unsupported(_)) => ToolResult {
                                value: json!({"error": "tool_preflight_denied"}),
                                is_error: true,
                            },
                            Err(_) if !spec.read_only => {
                                return Err(Error::Uncertain(call.id.clone()))
                            }
                            Err(_) => ToolResult {
                                value: json!({"error": "checked_tool_failed"}),
                                is_error: true,
                            },
                        };
                        if serde_json::to_vec(&output)?.len() > self.limits.max_result_bytes {
                            output = ToolResult {
                                value: json!({"error": "result_too_large", "artifact_required": true}),
                                is_error: true,
                            };
                        }
                        self.emit(ctx, "tool_end", json!({"invocation_id": call.id, "is_error": output.is_error, "result": output}))?;
                        output
                    }
                } else {
                    ToolResult {
                        value: json!({"error": "unknown_tool"}),
                        is_error: true,
                    }
                };
                self.append(ctx, history, self.host.result_message(call, &result)?)?;
                steering_pending.extend(ControlQueue::drain(&self.controls.steering)?);
            }
            self.emit(ctx, "turn_end", json!({"turn": turn}))?;
            let mut queued = steering_pending;
            queued.extend(ControlQueue::drain(&self.controls.steering)?);
            if no_calls && queued.is_empty() {
                queued.extend(ControlQueue::drain(&self.controls.follow_up)?);
            }
            if no_calls && queued.is_empty() {
                ctx.check()?;
                let repair = tokio::select! {
                    _ = ctx.cancelled() => return Err(Error::Cancelled),
                    result = self.host.accept_completion(ctx, history) => result?,
                };
                if let Some(text) = repair {
                    self.append(ctx, history, self.host.user_message(&text)?)?;
                    continue;
                }
                ctx.check()?;
                return Ok(());
            }
            for text in queued {
                self.append(ctx, history, self.host.user_message(&text)?)?;
            }
        }
        Err(Error::Invalid(
            "turn limit reached without accepted completion".into(),
        ))
    }
}
