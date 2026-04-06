//! Amazon Bedrock Converse / ConverseStream。

use crate::ProviderConfig;
use anycode_core::prelude::*;
use anycode_core::ANYCODE_TOOL_CALLS_METADATA_KEY;
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_bedrockruntime::types::{
    ContentBlock, ContentBlockDelta, ContentBlockStart, ConversationRole, InferenceConfiguration,
    Message as BedrockMessage, StopReason, SystemContentBlock, Tool, ToolConfiguration,
    ToolInputSchema, ToolResultBlock, ToolResultContentBlock, ToolSpecification, ToolUseBlock,
};
use aws_sdk_bedrockruntime::Client;
use aws_smithy_types::{Document, Number};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::error;
use uuid::Uuid;

pub struct BedrockClient {
    inner: Client,
}

impl BedrockClient {
    pub async fn from_provider_config(cfg: &ProviderConfig) -> Result<Self, CoreError> {
        let mut loader = aws_config::defaults(BehaviorVersion::latest());
        if let Some(ref u) = cfg.base_url {
            let t = u.trim();
            if !t.is_empty() {
                loader = loader.endpoint_url(t);
            }
        }
        let sdk = loader.load().await;
        Ok(Self {
            inner: Client::new(&sdk),
        })
    }
}

fn serde_json_value_to_document(v: serde_json::Value) -> Result<Document, CoreError> {
    Ok(match v {
        serde_json::Value::Null => Document::Null,
        serde_json::Value::Bool(b) => Document::Bool(b),
        serde_json::Value::Number(n) => {
            let num = if let Some(i) = n.as_i64() {
                if i < 0 {
                    Number::NegInt(i)
                } else {
                    Number::PosInt(i as u64)
                }
            } else if let Some(u) = n.as_u64() {
                Number::PosInt(u)
            } else {
                Number::Float(n.as_f64().unwrap_or(0.0))
            };
            Document::Number(num)
        }
        serde_json::Value::String(s) => Document::String(s),
        serde_json::Value::Array(a) => {
            let mut v = Vec::with_capacity(a.len());
            for x in a {
                v.push(serde_json_value_to_document(x)?);
            }
            Document::Array(v)
        }
        serde_json::Value::Object(o) => {
            let mut m = std::collections::HashMap::new();
            for (k, val) in o {
                m.insert(k, serde_json_value_to_document(val)?);
            }
            Document::Object(m)
        }
    })
}

fn document_to_json_value(d: &Document) -> serde_json::Value {
    match d {
        Document::Null => serde_json::Value::Null,
        Document::Bool(b) => serde_json::Value::Bool(*b),
        Document::Number(n) => match *n {
            Number::PosInt(u) => serde_json::Value::Number(u.into()),
            Number::NegInt(i) => serde_json::Value::Number(i.into()),
            Number::Float(f) => serde_json::Number::from_f64(f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
        },
        Document::String(s) => serde_json::Value::String(s.clone()),
        Document::Array(items) => serde_json::Value::Array(
            items.iter().map(document_to_json_value).collect(),
        ),
        Document::Object(map) => {
            let mut o = serde_json::Map::new();
            for (k, v) in map {
                o.insert(k.clone(), document_to_json_value(v));
            }
            serde_json::Value::Object(o)
        }
    }
}

fn build_tool_config(tools: Vec<ToolSchema>) -> Result<ToolConfiguration, CoreError> {
    let mut specs = Vec::with_capacity(tools.len());
    for t in tools {
        let doc = serde_json_value_to_document(t.input_schema)?;
        let spec = ToolSpecification::builder()
            .name(t.name)
            .description(t.description)
            .input_schema(ToolInputSchema::Json(doc))
            .build()
            .map_err(|e| CoreError::LLMError(format!("ToolSpecification: {}", e)))?;
        specs.push(Tool::ToolSpec(spec));
    }
    ToolConfiguration::builder()
        .set_tools(Some(specs))
        .build()
        .map_err(|e| CoreError::LLMError(format!("ToolConfiguration: {}", e)))
}

fn user_text_message(text: String) -> BedrockMessage {
    BedrockMessage::builder()
        .role(ConversationRole::User)
        .content(ContentBlock::Text(text))
        .build()
        .expect("bedrock user message")
}

fn assistant_blocks(parts: Vec<ContentBlock>) -> Result<BedrockMessage, CoreError> {
    let mut b = BedrockMessage::builder().role(ConversationRole::Assistant);
    for p in parts {
        b = b.content(p);
    }
    b.build()
        .map_err(|e| CoreError::LLMError(format!("assistant message: {}", e)))
}

fn tool_result_message(tool_use_id: String, content: String, is_error: bool) -> BedrockMessage {
    let status = if is_error {
        Some(aws_sdk_bedrockruntime::types::ToolResultStatus::Error)
    } else {
        Some(aws_sdk_bedrockruntime::types::ToolResultStatus::Success)
    };
    let block = ToolResultBlock::builder()
        .tool_use_id(tool_use_id)
        .content(
            ToolResultContentBlock::Text(content), // aws sdk uses Text variant
        )
        .set_status(status)
        .build()
        .expect("tool result");
    BedrockMessage::builder()
        .role(ConversationRole::User)
        .content(ContentBlock::ToolResult(block))
        .build()
        .expect("tool result msg")
}

fn convert_anycode_messages(
    messages: Vec<Message>,
) -> Result<(Vec<SystemContentBlock>, Vec<BedrockMessage>), CoreError> {
    let mut system: Vec<SystemContentBlock> = Vec::new();
    let mut out: Vec<BedrockMessage> = Vec::new();

    for msg in messages {
        if msg.role == MessageRole::Assistant {
            let mut parts: Vec<ContentBlock> = vec![];
            if let MessageContent::Text(t) = &msg.content {
                if !t.is_empty() {
                    parts.push(ContentBlock::Text(t.clone()));
                }
            }
            if let Some(v) = msg.metadata.get(ANYCODE_TOOL_CALLS_METADATA_KEY) {
                if let Ok(calls) = serde_json::from_value::<Vec<ToolCall>>(v.clone()) {
                    for c in calls {
                        let input = serde_json_value_to_document(c.input)?;
                        let tu = ToolUseBlock::builder()
                            .tool_use_id(c.id)
                            .name(c.name)
                            .input(input)
                            .build()
                            .map_err(|e| CoreError::LLMError(format!("ToolUseBlock: {}", e)))?;
                        parts.push(ContentBlock::ToolUse(tu));
                    }
                }
            }
            if parts.is_empty() {
                parts.push(ContentBlock::Text(String::new()));
            }
            out.push(assistant_blocks(parts)?);
            continue;
        }

        match msg.role {
            MessageRole::System => {
                if let MessageContent::Text(t) = msg.content {
                    if !t.is_empty() {
                        system.push(SystemContentBlock::Text(t));
                    }
                }
            }
            MessageRole::User => {
                if let MessageContent::Text(text) = msg.content {
                    out.push(user_text_message(text));
                }
            }
            MessageRole::Tool => {
                if let MessageContent::ToolResult {
                    tool_use_id,
                    content,
                    is_error,
                } = msg.content
                {
                    out.push(tool_result_message(tool_use_id, content, is_error));
                }
            }
            MessageRole::Assistant => {}
        }
    }

    Ok((system, out))
}

fn bedrock_message_to_response(
    msg: &BedrockMessage,
    usage: Option<aws_sdk_bedrockruntime::types::TokenUsage>,
) -> Result<LLMResponse, CoreError> {
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    for block in msg.content() {
        match block {
            ContentBlock::Text(t) => text.push_str(t),
            ContentBlock::ToolUse(tu) => {
                let input = document_to_json_value(tu.input());
                tool_calls.push(ToolCall {
                    id: tu.tool_use_id().to_string(),
                    name: tu.name().to_string(),
                    input,
                });
            }
            _ => {}
        }
    }
    let (in_tok, out_tok) = usage
        .map(|u| (u.input_tokens.max(0) as u32, u.output_tokens.max(0) as u32))
        .unwrap_or((0, 0));
    Ok(LLMResponse {
        message: Message {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text(text),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        },
        tool_calls,
        usage: Usage {
            input_tokens: in_tok,
            output_tokens: out_tok,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        },
    })
}

#[async_trait]
impl LLMClient for BedrockClient {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        let (system, msgs) = convert_anycode_messages(messages)?;
        let model_id = config.model.clone();
        if model_id.trim().is_empty() {
            return Err(CoreError::LLMError("Bedrock 须配置 model（foundation model id）".into()));
        }

        let mut inf = InferenceConfiguration::builder();
        inf = inf.max_tokens(config.max_tokens.unwrap_or(4096) as i32);
        if let Some(t) = config.temperature {
            inf = inf.temperature(t);
        }
        let inference = inf.build();

        let mut req = self
            .inner
            .converse()
            .model_id(&model_id)
            .inference_config(inference);

        for s in system {
            req = req.system(s);
        }
        for m in msgs {
            req = req.messages(m);
        }
        if !tools.is_empty() {
            req = req.tool_config(build_tool_config(tools)?);
        }

        let out = req
            .send()
            .await
            .map_err(|e| CoreError::LLMError(format!("Bedrock Converse: {}", e)))?;

        if matches!(out.stop_reason(), &StopReason::GuardrailIntervened) {
            return Err(CoreError::LLMError(
                "Bedrock: GuardrailIntervened".to_string(),
            ));
        }

        let usage = out.usage().cloned();
        let body = out
            .output
            .ok_or_else(|| CoreError::LLMError("Bedrock: empty output".into()))?;
        let msg = match body {
            aws_sdk_bedrockruntime::types::ConverseOutput::Message(m) => m,
            _ => {
                return Err(CoreError::LLMError(
                    "Bedrock: unexpected output variant".into(),
                ));
            }
        };

        bedrock_message_to_response(&msg, usage)
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<mpsc::Receiver<StreamEvent>, CoreError> {
        let (system, msgs) = convert_anycode_messages(messages)?;
        let model_id = config.model.clone();
        if model_id.trim().is_empty() {
            return Err(CoreError::LLMError("Bedrock 须配置 model".into()));
        }

        let mut inf = InferenceConfiguration::builder();
        inf = inf.max_tokens(config.max_tokens.unwrap_or(4096) as i32);
        if let Some(t) = config.temperature {
            inf = inf.temperature(t);
        }
        let inference = inf.build();

        let mut req = self
            .inner
            .converse_stream()
            .model_id(&model_id)
            .inference_config(inference);

        for s in system {
            req = req.system(s);
        }
        for m in msgs {
            req = req.messages(m);
        }
        if !tools.is_empty() {
            req = req.tool_config(build_tool_config(tools)?);
        }

        let out = req
            .send()
            .await
            .map_err(|e| CoreError::LLMError(format!("Bedrock ConverseStream: {}", e)))?;

        let mut stream = out.stream;
        let (tx, rx) = mpsc::channel(128);

        tokio::spawn(async move {
            #[derive(Default)]
            struct ToolAcc {
                id: String,
                name: String,
                input_json: String,
            }
            let mut tool_by_index: HashMap<i32, ToolAcc> = HashMap::new();

            loop {
                match stream.recv().await {
                    Ok(Some(
                        aws_sdk_bedrockruntime::types::ConverseStreamOutput::ContentBlockStart(
                            ev,
                        ),
                    )) => {
                        if let Some(start) = ev.start() {
                            let idx = ev.content_block_index();
                            match start {
                                ContentBlockStart::ToolUse(tus) => {
                                    tool_by_index.insert(
                                        idx,
                                        ToolAcc {
                                            id: tus.tool_use_id().to_string(),
                                            name: tus.name().to_string(),
                                            input_json: String::new(),
                                        },
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(Some(
                        aws_sdk_bedrockruntime::types::ConverseStreamOutput::ContentBlockDelta(ev),
                    )) => {
                        if let Some(delta) = ev.delta() {
                            let idx = ev.content_block_index();
                            match delta {
                                ContentBlockDelta::Text(t) => {
                                    let _ = tx.send(StreamEvent::Delta(t.clone())).await;
                                }
                                ContentBlockDelta::ToolUse(d) => {
                                    if let Some(acc) = tool_by_index.get_mut(&idx) {
                                        acc.input_json.push_str(d.input());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    Ok(Some(
                        aws_sdk_bedrockruntime::types::ConverseStreamOutput::ContentBlockStop(
                            ev,
                        ),
                    )) => {
                        let idx = ev.content_block_index();
                        if let Some(acc) = tool_by_index.remove(&idx) {
                            if !acc.id.is_empty() && !acc.name.is_empty() {
                                let input: serde_json::Value =
                                    serde_json::from_str(&acc.input_json).unwrap_or_else(|_| {
                                        if acc.input_json.is_empty() {
                                            serde_json::json!({})
                                        } else {
                                            serde_json::Value::String(acc.input_json.clone())
                                        }
                                    });
                                let _ = tx
                                    .send(StreamEvent::ToolCall(ToolCall {
                                        id: acc.id,
                                        name: acc.name,
                                        input,
                                    }))
                                    .await;
                            }
                        }
                    }
                    Ok(Some(aws_sdk_bedrockruntime::types::ConverseStreamOutput::MessageStop(_))) => {
                        let _ = tx.send(StreamEvent::Done).await;
                        break;
                    }
                    Ok(Some(_)) => {}
                    Ok(None) => {
                        let _ = tx.send(StreamEvent::Done).await;
                        break;
                    }
                    Err(e) => {
                        error!("Bedrock stream: {}", e);
                        let _ = tx.send(StreamEvent::Done).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }
}
