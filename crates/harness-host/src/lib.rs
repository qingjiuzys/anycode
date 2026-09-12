//! Lossless bridge to the existing anyCode provider/message contracts.
//! CheckedExecutor is mandatory: there is intentionally no bare Tool::execute adapter.
#![forbid(unsafe_code)]
pub mod graph_adapter;
use anycode_core::{
    LLMClient, Message, MessageContent, MessageRole, ModelConfig, StreamEvent, ToolCall,
    ToolSchema, ANYCODE_REASONING_CONTENT_METADATA_KEY, ANYCODE_TOOL_CALLS_METADATA_KEY,
};
use anycode_harness_core::{
    events::PreviewBus,
    kernel::Host,
    types::{Hop, Invocation, ProviderMessage, ToolResult, ToolSpec, Usage},
    Error, Result, RunContext,
};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};
use uuid::Uuid;
#[async_trait]
pub trait CheckedExecutor: Send + Sync {
    async fn invoke(&self, ctx: &RunContext, call: &Invocation) -> Result<ToolResult>;
    async fn transform(&self, ctx: &RunContext, history: &[Message]) -> Result<Vec<Message>>;
    async fn completion(&self, ctx: &RunContext, history: &[Message]) -> Result<Option<String>>;
}
pub struct AnyCodeHost {
    pub llm: Arc<dyn LLMClient>,
    pub model: ModelConfig,
    pub registry: Vec<ToolSpec>,
    pub executor: Arc<dyn CheckedExecutor>,
    pub streaming: bool,
    pub previews: PreviewBus,
}
fn message(role: MessageRole, content: MessageContent) -> Message {
    Message {
        id: Uuid::new_v4(),
        role,
        content,
        timestamp: chrono::Utc::now(),
        metadata: HashMap::new(),
    }
}
fn decode(messages: &[Value]) -> Result<Vec<Message>> {
    messages
        .iter()
        .cloned()
        .map(|v| serde_json::from_value(v).map_err(Error::from))
        .collect()
}
#[async_trait]
impl Host for AnyCodeHost {
    fn tools(&self) -> Result<Vec<ToolSpec>> {
        Ok(self.registry.clone())
    }
    async fn infer(
        &self,
        ctx: &RunContext,
        messages: Vec<ProviderMessage>,
        tools: Vec<ToolSpec>,
    ) -> Result<Hop> {
        ctx.check()?;
        let messages = decode(&messages)?;
        let schemas = tools
            .into_iter()
            .map(|s| ToolSchema {
                name: s.name,
                description: s.description,
                input_schema: s.input_schema,
            })
            .collect();
        if !self.streaming {
            let mut response = self
                .llm
                .chat(messages, schemas, &self.model)
                .await
                .map_err(|e| Error::Host(format!("provider request failed: {e}")))?;
            // Preserve all native metadata, including vision/reasoning/provider IDs.
            if !response.tool_calls.is_empty() {
                response.message.metadata.insert(
                    ANYCODE_TOOL_CALLS_METADATA_KEY.into(),
                    serde_json::to_value(&response.tool_calls)?,
                );
            }
            return Ok(Hop {
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
                usage: Some(Usage {
                    input_tokens: response.usage.input_tokens as u64,
                    output_tokens: response.usage.output_tokens as u64,
                    cache_creation_tokens: response.usage.cache_creation_tokens.unwrap_or(0) as u64,
                    cache_read_tokens: response.usage.cache_read_tokens.unwrap_or(0) as u64,
                }),
            });
        }
        let mut stream = self
            .llm
            .chat_stream(messages, schemas, &self.model)
            .await
            .map_err(|e| Error::Host(format!("provider stream open failed: {e}")))?;
        let mut text = String::new();
        let mut reasoning = String::new();
        let mut calls = vec![];
        let mut usage = None;
        let mut total = 0usize;
        let mut done = false;
        let mut seen = BTreeMap::new();
        while let Some(event) = stream.recv().await {
            ctx.check()?;
            match event {
                StreamEvent::Delta(delta) => {
                    total = total.saturating_add(delta.len());
                    text.push_str(&delta);
                    self.previews.publish(json!({"run_id":ctx.id(),"kind":"text_delta","text":delta,"provisional":true}));
                }
                StreamEvent::Reasoning(delta) => {
                    total = total.saturating_add(delta.len());
                    reasoning.push_str(&delta);
                }
                StreamEvent::ToolCall(call) => {
                    total = total.saturating_add(serde_json::to_vec(&call)?.len());
                    if seen.insert(call.id.clone(), ()).is_some() {
                        return Err(Error::Invalid("duplicate streamed tool id".into()));
                    }
                    calls.push(call);
                }
                StreamEvent::Usage(value) => {
                    usage = Some(Usage {
                        input_tokens: value.input_tokens as u64,
                        output_tokens: value.output_tokens as u64,
                        cache_creation_tokens: value.cache_creation_tokens.unwrap_or(0) as u64,
                        cache_read_tokens: value.cache_read_tokens.unwrap_or(0) as u64,
                    });
                }
                StreamEvent::Failed(reason) => {
                    return Err(Error::Host(format!(
                        "provider stream failed; partial response discarded: {reason}"
                    )))
                }
                // Do not synthesize undocumented prefix-hash metadata. Disabling this
                // optimization retains correctness by resending full native history.
                StreamEvent::ResponseId { .. } => {}
                StreamEvent::Done => {
                    done = true;
                    break;
                }
            }
            if total > 4 * 1024 * 1024 || calls.len() > 32 {
                return Err(Error::Invalid("stream output bounds".into()));
            }
        }
        if !done {
            return Err(Error::Host(
                "provider stream ended without Done; no tools executed".into(),
            ));
        }
        let mut msg = message(MessageRole::Assistant, MessageContent::Text(text));
        if !reasoning.is_empty() {
            msg.metadata.insert(
                ANYCODE_REASONING_CONTENT_METADATA_KEY.into(),
                reasoning.into(),
            );
        }
        if !calls.is_empty() {
            msg.metadata.insert(
                ANYCODE_TOOL_CALLS_METADATA_KEY.into(),
                serde_json::to_value(&calls)?,
            );
        }
        Ok(Hop {
            assistant: serde_json::to_value(msg)?,
            calls: calls
                .into_iter()
                .map(|c: ToolCall| Invocation {
                    id: c.id,
                    name: c.name,
                    arguments: c.input,
                })
                .collect(),
            usage,
        })
    }
    async fn invoke_checked(&self, ctx: &RunContext, call: &Invocation) -> Result<ToolResult> {
        self.executor.invoke(ctx, call).await
    }
    fn user_message(&self, text: &str) -> Result<ProviderMessage> {
        Ok(serde_json::to_value(message(
            MessageRole::User,
            MessageContent::Text(text.into()),
        ))?)
    }
    fn result_message(&self, call: &Invocation, result: &ToolResult) -> Result<ProviderMessage> {
        Ok(serde_json::to_value(message(
            MessageRole::Tool,
            MessageContent::ToolResult {
                tool_use_id: call.id.clone(),
                content: serde_json::to_string(&result.value)?,
                is_error: result.is_error,
            },
        ))?)
    }
    async fn transform_context(
        &self,
        ctx: &RunContext,
        messages: &[ProviderMessage],
    ) -> Result<Vec<ProviderMessage>> {
        self.executor
            .transform(ctx, &decode(messages)?)
            .await?
            .into_iter()
            .map(|m| serde_json::to_value(m).map_err(Error::from))
            .collect()
    }
    async fn accept_completion(
        &self,
        ctx: &RunContext,
        history: &[ProviderMessage],
    ) -> Result<Option<String>> {
        self.executor.completion(ctx, &decode(history)?).await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn serialization_retains_reasoning_tool_ids_and_image_metadata() {
        let mut msg = message(MessageRole::Assistant, MessageContent::Text("text".into()));
        msg.metadata.insert(
            ANYCODE_REASONING_CONTENT_METADATA_KEY.into(),
            "native reasoning".into(),
        );
        msg.metadata
            .insert("custom_provider_signature".into(), "signature".into());
        let decoded = decode(&[serde_json::to_value(&msg).unwrap()]).unwrap();
        assert_eq!(decoded[0].metadata, msg.metadata);
    }
}
