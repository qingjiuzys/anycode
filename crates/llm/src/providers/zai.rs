//! z.ai (BigModel/智谱) provider implementation.
//!
//! OpenAI 兼容 `chat/completions`：支持 `tools` / `tool_calls`（与 OpenAI Chat Completions 对齐）。

use crate::normalize_provider_id;
use anycode_core::prelude::*;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, warn};
use uuid::Uuid;

pub(crate) fn is_retryable_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

pub(crate) fn retry_delay_ms(attempt: u32) -> u64 {
    const BASE: u64 = 500;
    const CAP: u64 = 10_000;
    let exp = 2u64.saturating_pow(attempt.saturating_sub(1));
    std::cmp::min(CAP, BASE.saturating_mul(exp))
}

/// z.ai Coding 套餐默认 endpoint（与 `ZaiClient` 默认一致）
pub const ZAI_DEFAULT_CODING_ENDPOINT: &str =
    "https://api.z.ai/api/coding/paas/v4/chat/completions";

const DEFAULT_API_TIMEOUT_MS: u64 = 600_000;
const DEFAULT_MAX_RETRIES: u32 = 10;

/// 向导 / CLI 展示用的模型目录（单一事实来源）
#[derive(Debug, Clone, Copy)]
pub struct ZaiModelCatalogEntry {
    pub api_name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub wizard_line: &'static str,
}

pub const ZAI_MODEL_CATALOG: &[ZaiModelCatalogEntry] = &[
    ZaiModelCatalogEntry {
        api_name: "glm-5",
        display_name: "GLM-5 (Coding 默认)",
        description: "z.ai Coding 套餐常用默认模型",
        wizard_line: "GLM-5 - Coding 默认，通用编码与对话",
    },
    ZaiModelCatalogEntry {
        api_name: "glm-4.7",
        display_name: "GLM-4.7",
        description: "较强通用能力，适合复杂任务",
        wizard_line: "GLM-4.7 - 较强通用能力",
    },
    ZaiModelCatalogEntry {
        api_name: "glm-4",
        display_name: "GLM-4 (最强大)",
        description: "最强大的模型，适合复杂任务",
        wizard_line: "GLM-4 - 最强大，128K 上下文，适合复杂任务",
    },
    ZaiModelCatalogEntry {
        api_name: "glm-4-air",
        display_name: "GLM-4-Air (轻量)",
        description: "轻量级模型，性价比高",
        wizard_line: "GLM-4-Air - 轻量级，性价比高",
    },
    ZaiModelCatalogEntry {
        api_name: "glm-4-flash",
        display_name: "GLM-4-Flash (快速)",
        description: "极速响应，适合简单任务",
        wizard_line: "GLM-4-Flash - 极速响应，适合简单任务",
    },
    ZaiModelCatalogEntry {
        api_name: "glm-3-turbo",
        display_name: "GLM-3-Turbo (超快)",
        description: "超快速度，成本最低",
        wizard_line: "GLM-3-Turbo - 超快速度，成本最低",
    },
];

/// 配置文件中可序列化的 z.ai 模型 id（与 OpenAPI `model` 字段一致）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ZaiModel {
    #[serde(rename = "glm-5")]
    GLM5,
    #[serde(rename = "glm-4.7")]
    GLM47,
    #[serde(rename = "glm-4")]
    GLM4,
    #[serde(rename = "glm-4-air")]
    GLM4Air,
    #[serde(rename = "glm-4-flash")]
    GLM4Flash,
    #[serde(rename = "glm-3-turbo")]
    GLM3Turbo,
}

impl ZaiModel {
    pub fn api_name(&self) -> &'static str {
        match self {
            ZaiModel::GLM5 => "glm-5",
            ZaiModel::GLM47 => "glm-4.7",
            ZaiModel::GLM4 => "glm-4",
            ZaiModel::GLM4Air => "glm-4-air",
            ZaiModel::GLM4Flash => "glm-4-flash",
            ZaiModel::GLM3Turbo => "glm-3-turbo",
        }
    }

    pub fn from_api_name(s: &str) -> Option<Self> {
        match s {
            "glm-5" => Some(ZaiModel::GLM5),
            "glm-4.7" => Some(ZaiModel::GLM47),
            "glm-4" => Some(ZaiModel::GLM4),
            "glm-4-air" => Some(ZaiModel::GLM4Air),
            "glm-4-flash" => Some(ZaiModel::GLM4Flash),
            "glm-3-turbo" => Some(ZaiModel::GLM3Turbo),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &'static str {
        ZAI_MODEL_CATALOG
            .iter()
            .find(|e| e.api_name == self.api_name())
            .map(|e| e.display_name)
            .unwrap_or(self.api_name())
    }

    pub fn description(&self) -> &'static str {
        ZAI_MODEL_CATALOG
            .iter()
            .find(|e| e.api_name == self.api_name())
            .map(|e| e.description)
            .unwrap_or("")
    }
}

/// z.ai Client（OpenAI 兼容 chat/completions 格式）
pub struct ZaiClient {
    client: Client,
    api_key: String,
    base_url: String,
    model: String,
    /// 与 `ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN` 叠加：任一为真则首轮可发 `tool_choice: required`。
    tool_choice_first_turn: bool,
}

impl ZaiClient {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        let model = model.unwrap_or_else(|| "glm-5".to_string());
        Self {
            client: build_http_client(),
            api_key,
            base_url: ZAI_DEFAULT_CODING_ENDPOINT.to_string(),
            model,
            tool_choice_first_turn: false,
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    pub fn with_tool_choice_first_turn(mut self, enabled: bool) -> Self {
        self.tool_choice_first_turn = enabled;
        self
    }
}

fn configured_api_timeout_ms() -> u64 {
    std::env::var("API_TIMEOUT_MS")
        .ok()
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|v| *v >= 1_000)
        .unwrap_or(DEFAULT_API_TIMEOUT_MS)
}

fn build_http_client() -> Client {
    Client::builder()
        .timeout(Duration::from_millis(configured_api_timeout_ms()))
        .build()
        .unwrap_or_else(|_| Client::new())
}

fn sanitize_header_token(raw: &str, provider_name: &str) -> Result<String, CoreError> {
    let token = raw.trim();
    if token.is_empty() {
        return Err(CoreError::LLMError(format!(
            "{provider_name} api_key is empty after trimming whitespace"
        )));
    }
    if token
        .chars()
        .any(|c| c.is_control() || matches!(c, '\u{7f}'))
    {
        return Err(CoreError::LLMError(format!(
            "{provider_name} api_key contains control characters; please reconfigure the key (remove newline/hidden chars)"
        )));
    }
    Ok(token.to_string())
}

fn normalize_zai_base_url(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return ZAI_DEFAULT_CODING_ENDPOINT.to_string();
    }
    if trimmed.ends_with("/chat/completions") {
        return trimmed.to_string();
    }
    let mut s = trimmed.trim_end_matches('/').to_string();
    if s.ends_with("/v4") {
        s.push_str("/chat/completions");
        return s;
    }
    if s.ends_with("/coding/paas") {
        s.push_str("/v4/chat/completions");
        return s;
    }
    if s.ends_with("/api") {
        s.push_str("/coding/paas/v4/chat/completions");
        return s;
    }
    if s.contains("open.bigmodel.cn") || s.contains("api.z.ai") {
        s.push_str("/api/coding/paas/v4/chat/completions");
        return s;
    }
    s
}

fn provider_label_from_config(config: &ModelConfig) -> String {
    match &config.provider {
        LLMProvider::Custom(s) => {
            let n = normalize_provider_id(s);
            if n.is_empty() {
                "openai-compatible".to_string()
            } else {
                n
            }
        }
        LLMProvider::OpenAI => "openai".to_string(),
        LLMProvider::Anthropic => "anthropic".to_string(),
        LLMProvider::Local => "local".to_string(),
    }
}

fn normalize_openai_compatible_base_url(raw: &str, provider_label: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return normalize_zai_base_url(trimmed);
    }
    // Google 常见误填为原生 Gemini 路径（/v1beta/models/...），这里自动归一到 OpenAI 兼容入口。
    if provider_label == "google" && trimmed.contains("generativelanguage.googleapis.com") {
        let s = trimmed.trim_end_matches('/');
        if s.ends_with("/v1beta/openai/chat/completions") {
            return s.to_string();
        }
        if s.contains("/v1beta/models/") || s.ends_with("/v1beta") || s.ends_with("/v1beta/openai")
        {
            return "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
                .to_string();
        }
    }
    normalize_zai_base_url(trimmed)
}

#[derive(Debug, Serialize)]
struct ZaiRequestBody {
    model: String,
    messages: Vec<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    /// GLM：与官方 SDK 默认一致；关闭设 `ANYCODE_ZAI_THINKING=0`。
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ZaiResponse {
    #[allow(dead_code)] // OpenAI 兼容 JSON 的 id，当前未向上返回
    id: String,
    choices: Vec<ZaiChoice>,
    usage: ZaiUsage,
}

#[derive(Debug, Deserialize)]
struct ZaiChoice {
    message: ZaiMessageContent,
    /// 少数网关把思考放在 choice 层而非 message 内。
    #[serde(default)]
    reasoning_content: Option<String>,
    #[allow(dead_code)] // 部分网关返回；仅用于 tracing
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ZaiMessageContent {
    /// OpenAI 兼容：字符串，或 `[{"type":"text","text":"…"}]`（GLM 常见）。
    #[serde(default)]
    content: Option<Value>,
    /// GLM「思考链」：开启 thinking 时常仅有此字段、`content` 为空，须并入 assistant 正文否则 TUI 无总结。
    #[serde(default)]
    reasoning_content: Option<String>,
    /// 部分响应里思考为嵌套对象（与请求体 `thinking` 对称）。
    #[serde(default)]
    thinking: Option<Value>,
    #[serde(default)]
    tool_calls: Option<Vec<ZaiToolCall>>,
}

fn zai_value_content_to_string(content: &Option<Value>) -> String {
    match content {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Object(map)) => {
            if let Some(t) = map.get("text").and_then(|x| x.as_str()) {
                return t.to_string();
            }
            if let Some(t) = map.get("content").and_then(|x| x.as_str()) {
                return t.to_string();
            }
            serde_json::to_string(map).unwrap_or_default()
        }
        Some(Value::Array(parts)) => {
            let mut out = String::new();
            for item in parts {
                if let Some(t) = item.get("text").and_then(|x| x.as_str()) {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(t);
                } else if let Some(s) = item.as_str() {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(s);
                }
            }
            out
        }
        Some(other) => other
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| other.to_string()),
    }
}

fn extract_tagged_blocks(s: &str, open: &str, close: &str) -> (String, String) {
    if open.is_empty() || close.is_empty() {
        return (s.to_string(), String::new());
    }
    let mut visible = String::new();
    let mut extracted = String::new();
    let mut rest = s;
    while let Some(i) = rest.find(open) {
        visible.push_str(&rest[..i]);
        rest = &rest[i + open.len()..];
        match rest.find(close) {
            Some(j) => {
                let inner = rest[..j].trim();
                if !inner.is_empty() {
                    if !extracted.is_empty() {
                        extracted.push_str("\n\n");
                    }
                    extracted.push_str(inner);
                }
                rest = &rest[j + close.len()..];
            }
            None => {
                visible.push_str(open);
                visible.push_str(rest);
                return (visible, extracted);
            }
        }
    }
    visible.push_str(rest);
    (visible, extracted)
}

/// GLM 偶发把推理放在 `<think>...</think>` 或文档所述 `redacted_thinking` 标签内。
fn zai_split_thinking_markers(s: &str) -> (String, String) {
    let mut visible = s.to_string();
    let mut extracted = String::new();
    for (open, close) in [
        ("<think>", "</think>"),
        ("<redacted_thinking>", "</redacted_thinking>"),
    ] {
        let (v, r) = extract_tagged_blocks(&visible, open, close);
        visible = v;
        if !r.is_empty() {
            if !extracted.is_empty() {
                extracted.push_str(
                    "

",
                );
            }
            extracted.push_str(&r);
        }
    }
    (visible, extracted)
}

fn zai_thinking_object_to_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(m) => m
            .get("content")
            .or_else(|| m.get("text"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        _ => None,
    }
}

fn zai_collect_reasoning_parts(
    msg: &ZaiMessageContent,
    choice_level_reasoning: Option<&str>,
) -> String {
    let mut parts: Vec<String> = vec![];
    if let Some(ref r) = msg.reasoning_content {
        let t = r.trim();
        if !t.is_empty() {
            parts.push(r.clone());
        }
    }
    if let Some(ref th) = msg.thinking {
        if let Some(s) = zai_thinking_object_to_string(th) {
            let t = s.trim();
            if !t.is_empty() {
                parts.push(s);
            }
        }
    }
    if let Some(cf) = choice_level_reasoning {
        let t = cf.trim();
        if !t.is_empty() {
            parts.push(cf.to_string());
        }
    }
    parts.join("\n\n")
}

/// 合并可见正文：`content` + 各层 `reasoning` / `thinking` / choice 级 reasoning；无 tool 时写入会话。
///
/// **带 `tool_calls` 时**优先仅用 `content` 可见部分；若 `content` 为空则回退到 reasoning / 标签内文（避免工具轮次界面空白）。
fn zai_merge_assistant_visible_text(
    msg: &ZaiMessageContent,
    choice_level_reasoning: Option<&str>,
) -> String {
    let raw_content = zai_value_content_to_string(&msg.content);
    let (visible_from_tags, tagged_inner) = zai_split_thinking_markers(&raw_content);
    let from_content = visible_from_tags;
    let has_tools = msg
        .tool_calls
        .as_ref()
        .map(|t| !t.is_empty())
        .unwrap_or(false);

    let mut extra_r = zai_collect_reasoning_parts(msg, choice_level_reasoning);
    if !tagged_inner.is_empty() {
        if !extra_r.is_empty() {
            extra_r.push_str("\n\n");
        }
        extra_r.push_str(&tagged_inner);
    }

    if has_tools {
        let c = from_content.trim();
        if !c.is_empty() {
            return from_content;
        }
        let r = extra_r.trim();
        if !r.is_empty() {
            return extra_r;
        }
        return from_content;
    }

    let r = extra_r.trim();
    let c = from_content.trim();
    if c.is_empty() && !r.is_empty() {
        return r.to_string();
    }
    if !c.is_empty() && r.is_empty() {
        return from_content;
    }
    if c.is_empty() {
        return String::new();
    }
    format!("{from_content}\n\n{r}")
}

#[derive(Debug, Deserialize)]
struct ZaiToolCall {
    id: String,
    #[serde(default)]
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: ZaiToolFunction,
}

#[derive(Debug, Deserialize)]
struct ZaiToolFunction {
    name: String,
    #[serde(default)]
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct ZaiUsage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
    #[allow(dead_code)] // usage 扩展字段，与 OpenAI 对齐
    total_tokens: Option<u32>,
}

/// 是否为「首轮主对话」：仅 system + user，尚无 assistant/tool 历史（与 AgentRuntime 首包 LLM 请求一致）。
fn is_zai_first_agent_turn(messages: &[Message]) -> bool {
    messages.len() == 2
        && matches!(messages.first().map(|m| &m.role), Some(MessageRole::System))
        && matches!(messages.get(1).map(|m| &m.role), Some(MessageRole::User))
}

/// OpenAI 兼容 `tool_choice`。Claude Code 对 Anthropic 主路径传 `tool_choice: undefined`，依赖 Claude 的工具习惯；z.ai/GLM 在 `auto` 下常返回纯文本而不带 `tool_calls`，故支持用环境变量收紧（与 OpenAI `required` 语义一致）。
fn zai_thinking_body() -> Option<Value> {
    match std::env::var("ANYCODE_ZAI_THINKING").ok().as_deref() {
        Some("0") | Some("false") | Some("no") | Some("off") | Some("disabled") => None,
        _ => Some(json!({"type": "enabled"})),
    }
}

fn openai_compatible_thinking_body(provider_label: &str) -> Option<Value> {
    if provider_label == "z.ai" {
        return zai_thinking_body();
    }
    None
}

fn zai_tool_choice(
    messages: &[Message],
    tools_empty: bool,
    client_wants_first_turn_required: bool,
) -> Option<String> {
    if tools_empty {
        return None;
    }
    if let Ok(v) = std::env::var("ANYCODE_ZAI_TOOL_CHOICE") {
        let v = v.trim().to_lowercase();
        if v == "required" || v == "auto" {
            return Some(v);
        }
    }
    let env_first_turn = matches!(
        std::env::var("ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN")
            .ok()
            .as_deref(),
        Some("1") | Some("true") | Some("yes")
    );
    let first_turn_required = client_wants_first_turn_required || env_first_turn;
    if first_turn_required && is_zai_first_agent_turn(messages) {
        return Some("required".to_string());
    }
    Some("auto".to_string())
}

pub(crate) fn openai_tools_from_schemas(tools: &[ToolSchema]) -> Vec<Value> {
    tools
        .iter()
        .map(|t| {
            json!({
                "type": "function",
                "function": {
                    "name": t.name,
                    "description": t.description,
                    "parameters": t.input_schema
                }
            })
        })
        .collect()
}

fn message_text_for_openai(m: &Message) -> Result<String, CoreError> {
    match &m.content {
        MessageContent::Text(s) => Ok(s.clone()),
        MessageContent::ToolUse { name, input } => Ok(format!(
            "[Tool: {} input: {}]",
            name,
            serde_json::to_string(input).unwrap_or_default()
        )),
        MessageContent::ToolResult { content, .. } => Ok(content.clone()),
    }
}

/// 将 anyCode 消息转为 OpenAI Chat Completions `messages` JSON。
pub(crate) fn messages_to_openai_json(messages: Vec<Message>) -> Result<Vec<Value>, CoreError> {
    let mut out = Vec::with_capacity(messages.len());
    for msg in messages {
        match msg.role {
            MessageRole::System => {
                out.push(json!({
                    "role": "system",
                    "content": message_text_for_openai(&msg)?
                }));
            }
            MessageRole::User => {
                out.push(json!({
                    "role": "user",
                    "content": message_text_for_openai(&msg)?
                }));
            }
            MessageRole::Assistant => {
                let text = message_text_for_openai(&msg)?;
                if let Some(raw) = msg.metadata.get(ANYCODE_TOOL_CALLS_METADATA_KEY) {
                    let calls: Vec<ToolCall> =
                        serde_json::from_value(raw.clone()).map_err(|e| {
                            CoreError::LLMError(format!(
                                "invalid {} metadata: {}",
                                ANYCODE_TOOL_CALLS_METADATA_KEY, e
                            ))
                        })?;
                    let tool_calls: Vec<Value> = calls
                        .iter()
                        .map(|c| {
                            let args = serde_json::to_string(&c.input)
                                .unwrap_or_else(|_| "{}".to_string());
                            Ok(json!({
                                "id": c.id,
                                "type": "function",
                                "function": {
                                    "name": c.name,
                                    "arguments": args
                                }
                            })) as Result<Value, CoreError>
                        })
                        .collect::<Result<_, _>>()?;
                    let mut obj = serde_json::Map::new();
                    obj.insert("role".to_string(), json!("assistant"));
                    if text.is_empty() {
                        obj.insert("content".to_string(), Value::Null);
                    } else {
                        obj.insert("content".to_string(), json!(text));
                    }
                    obj.insert("tool_calls".to_string(), Value::Array(tool_calls));
                    out.push(Value::Object(obj));
                } else {
                    out.push(json!({
                        "role": "assistant",
                        "content": text
                    }));
                }
            }
            MessageRole::Tool => {
                let (tool_call_id, content) = match &msg.content {
                    MessageContent::ToolResult {
                        tool_use_id,
                        content,
                        ..
                    } => (tool_use_id.clone(), content.clone()),
                    other => {
                        return Err(CoreError::LLMError(format!(
                            "expected ToolResult for Tool role, got {:?}",
                            other
                        )));
                    }
                };
                out.push(json!({
                    "role": "tool",
                    "tool_call_id": tool_call_id,
                    "content": content
                }));
            }
        }
    }
    Ok(out)
}

fn parse_tool_calls_from_zai(msg: &ZaiMessageContent) -> Result<Vec<ToolCall>, CoreError> {
    let Some(raw) = &msg.tool_calls else {
        return Ok(vec![]);
    };
    let mut out = Vec::new();
    for tc in raw {
        let input: Value = if tc.function.arguments.trim().is_empty() {
            json!({})
        } else {
            serde_json::from_str(&tc.function.arguments)
                .unwrap_or_else(|_| json!({ "raw": tc.function.arguments.clone() }))
        };
        let _ = tc.call_type.as_deref(); // OpenAI uses "function"
        out.push(ToolCall {
            id: tc.id.clone(),
            name: tc.function.name.clone(),
            input,
        });
    }
    Ok(out)
}

fn convert_response(zai_response: ZaiResponse) -> Result<LLMResponse, CoreError> {
    let choice = zai_response
        .choices
        .first()
        .ok_or_else(|| CoreError::LLMError("z.ai: empty choices".to_string()))?;
    let content_str =
        zai_merge_assistant_visible_text(&choice.message, choice.reasoning_content.as_deref());
    if content_str.trim().is_empty() {
        warn!(
            finish_reason = ?choice.finish_reason,
            tool_calls_n = choice
                .message
                .tool_calls
                .as_ref()
                .map(|t| t.len())
                .unwrap_or(0),
            "z.ai: merged assistant visible text is empty"
        );
    }
    let tool_calls = parse_tool_calls_from_zai(&choice.message)?;

    Ok(LLMResponse {
        message: Message {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text(content_str),
            timestamp: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        },
        tool_calls,
        usage: Usage {
            input_tokens: zai_response.usage.prompt_tokens,
            output_tokens: zai_response.usage.completion_tokens,
            cache_creation_tokens: None,
            cache_read_tokens: None,
        },
    })
}

/// OpenAI Chat Completions 与 z.ai 兼容响应体共用反序列化与 [`convert_response`]。
#[cfg(feature = "openai")]
pub(crate) fn llm_response_from_openai_compatible_str(s: &str) -> Result<LLMResponse, CoreError> {
    let zai_response: ZaiResponse =
        serde_json::from_str(s).map_err(|e| CoreError::LLMError(e.to_string()))?;
    convert_response(zai_response)
}

#[async_trait]
impl LLMClient for ZaiClient {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Vec<ToolSchema>,
        config: &ModelConfig,
    ) -> Result<LLMResponse, CoreError> {
        let tool_choice = zai_tool_choice(&messages, tools.is_empty(), self.tool_choice_first_turn);
        let openai_messages = messages_to_openai_json(messages)?;

        let model = if config.model.trim().is_empty() {
            self.model.clone()
        } else {
            config.model.clone()
        };

        let tools_json = if tools.is_empty() {
            None
        } else {
            Some(openai_tools_from_schemas(&tools))
        };

        let provider_label = provider_label_from_config(config);
        if tools_json.is_some() {
            debug!(
                "{} request includes {} tools, tool_choice={:?}",
                provider_label,
                tools.len(),
                tool_choice
            );
        }

        let body = ZaiRequestBody {
            model,
            messages: openai_messages,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            stream: Some(false),
            tools: tools_json,
            tool_choice,
            thinking: openai_compatible_thinking_body(&provider_label),
        };

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| self.base_url.clone());
        let base_url = normalize_openai_compatible_base_url(&base_url, &provider_label);

        let auth_key = config
            .api_key
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(self.api_key.as_str());
        let auth_key = sanitize_header_token(auth_key, &provider_label)?;

        let max_retries: u32 = DEFAULT_MAX_RETRIES;
        let mut last_err: Option<String> = None;
        let mut response: Option<reqwest::Response> = None;
        for attempt in 1..=max_retries + 1 {
            let send_res = self
                .client
                .post(&base_url)
                .header("Authorization", format!("Bearer {}", auth_key))
                .json(&body)
                .send()
                .await;

            match send_res {
                Ok(resp) => {
                    let status = resp.status();
                    if status.is_success() {
                        response = Some(resp);
                        break;
                    }

                    let retry_after_ms = resp
                        .headers()
                        .get("retry-after")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.parse::<u64>().ok())
                        .map(|secs| secs.saturating_mul(1000));

                    let error_text = resp.text().await.unwrap_or_default();
                    let mut snippet = error_text.clone();
                    const MAX_ERR: usize = 2000;
                    if snippet.len() > MAX_ERR {
                        snippet.truncate(MAX_ERR);
                        snippet.push_str("...<truncated>");
                    }
                    last_err = Some(format!(
                        "{} API error: status={} url={} body={}",
                        provider_label,
                        status.as_u16(),
                        base_url,
                        if snippet.is_empty() {
                            "<empty>"
                        } else {
                            &snippet
                        }
                    ));

                    if attempt <= max_retries && is_retryable_status(status) {
                        let delay = retry_after_ms.unwrap_or_else(|| retry_delay_ms(attempt));
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    break;
                }
                Err(e) => {
                    let mut msg = e.to_string();
                    if e.is_timeout() {
                        msg = format!(
                            "{msg} · API_TIMEOUT_MS={}ms, try increasing it",
                            configured_api_timeout_ms()
                        );
                    }
                    last_err = Some(msg);
                    if attempt <= max_retries {
                        let delay = retry_delay_ms(attempt);
                        warn!(
                            "Retrying in {} seconds… (attempt {}/{}){}",
                            delay / 1000,
                            attempt,
                            max_retries,
                            if std::env::var("API_TIMEOUT_MS").is_ok() {
                                format!(
                                    " · API_TIMEOUT_MS={}ms, try increasing it",
                                    configured_api_timeout_ms()
                                )
                            } else {
                                String::new()
                            }
                        );
                        tokio::time::sleep(tokio::time::Duration::from_millis(delay)).await;
                        continue;
                    }
                    break;
                }
            }
        }

        let response = response.ok_or_else(|| {
            CoreError::LLMError(format!(
                "{} request failed after retries: {}",
                provider_label,
                last_err.unwrap_or_else(|| "unknown error".to_string())
            ))
        })?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(CoreError::LLMError(format!(
                "{} API error (no retry): status={} url={} body={}",
                provider_label,
                status.as_u16(),
                base_url,
                if error_text.is_empty() {
                    "<empty>"
                } else {
                    &error_text
                }
            )));
        }

        let zai_response: ZaiResponse = response
            .json()
            .await
            .map_err(|e| CoreError::LLMError(e.to_string()))?;

        convert_response(zai_response)
    }

    async fn chat_stream(
        &self,
        _messages: Vec<Message>,
        _tools: Vec<ToolSchema>,
        _config: &ModelConfig,
    ) -> Result<mpsc::Receiver<StreamEvent>, CoreError> {
        let (tx, rx) = mpsc::channel(1);
        tokio::spawn(async move {
            let _ = tx.send(StreamEvent::Done).await;
        });
        Ok(rx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn openai_messages_include_tool_calls_from_metadata() {
        let tc = ToolCall {
            id: "call_x".to_string(),
            name: "Echo".to_string(),
            input: json!({"a": 1}),
        };
        let mut meta = HashMap::new();
        meta.insert(
            ANYCODE_TOOL_CALLS_METADATA_KEY.to_string(),
            serde_json::to_value(vec![tc]).unwrap(),
        );
        let assistant = Message {
            id: Uuid::new_v4(),
            role: MessageRole::Assistant,
            content: MessageContent::Text(String::new()),
            timestamp: chrono::Utc::now(),
            metadata: meta,
        };
        let out = messages_to_openai_json(vec![assistant]).unwrap();
        let obj = out[0].as_object().unwrap();
        assert_eq!(obj.get("role").and_then(|v| v.as_str()), Some("assistant"));
        assert!(obj.get("tool_calls").is_some());
    }

    #[test]
    fn sanitize_header_token_trims_whitespace() {
        let out = sanitize_header_token("  sk-test-123  ", "z.ai").unwrap();
        assert_eq!(out, "sk-test-123");
    }

    #[test]
    fn sanitize_header_token_rejects_control_chars() {
        let err = sanitize_header_token("sk-test\n123", "z.ai").unwrap_err();
        assert!(err.to_string().contains("control characters"));
    }

    #[test]
    fn normalize_zai_base_url_appends_chat_completions_from_v4() {
        let got = normalize_zai_base_url("https://open.bigmodel.cn/api/coding/paas/v4");
        assert_eq!(
            got,
            "https://open.bigmodel.cn/api/coding/paas/v4/chat/completions"
        );
    }

    #[test]
    fn normalize_zai_base_url_keeps_full_endpoint() {
        let got = normalize_zai_base_url("https://api.z.ai/api/coding/paas/v4/chat/completions");
        assert_eq!(got, "https://api.z.ai/api/coding/paas/v4/chat/completions");
    }

    fn test_msg(role: MessageRole, text: &str) -> Message {
        Message {
            id: Uuid::new_v4(),
            role,
            content: MessageContent::Text(text.to_string()),
            timestamp: chrono::Utc::now(),
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn is_zai_first_agent_turn_only_system_user() {
        let ok = vec![
            test_msg(MessageRole::System, "s"),
            test_msg(MessageRole::User, "u"),
        ];
        assert!(is_zai_first_agent_turn(&ok));
        let bad = vec![
            test_msg(MessageRole::System, "s"),
            test_msg(MessageRole::User, "u"),
            test_msg(MessageRole::Assistant, "a"),
        ];
        assert!(!is_zai_first_agent_turn(&bad));
    }

    #[test]
    fn zai_tool_choice_no_tools_omits_choice() {
        let m = vec![
            test_msg(MessageRole::System, "s"),
            test_msg(MessageRole::User, "u"),
        ];
        assert_eq!(zai_tool_choice(&m, true, false), None);
    }

    #[test]
    fn zai_tool_choice_client_flag_first_turn_required() {
        let m = vec![
            test_msg(MessageRole::System, "s"),
            test_msg(MessageRole::User, "u"),
        ];
        assert_eq!(
            zai_tool_choice(&m, false, true),
            Some("required".to_string())
        );
    }

    #[test]
    fn parses_response_with_tool_calls() {
        let json_str = r#"{
            "id": "1",
            "choices": [{
                "message": {
                    "content": null,
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "FileRead", "arguments": "{\"file_path\":\"/tmp/a\"}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 2}
        }"#;
        let z: ZaiResponse = serde_json::from_str(json_str).unwrap();
        let r = convert_response(z).unwrap();
        assert_eq!(r.tool_calls.len(), 1);
        assert_eq!(r.tool_calls[0].name, "FileRead");
        assert!(r.tool_calls[0].input.get("file_path").is_some());
    }

    #[test]
    fn parses_response_reasoning_when_content_empty() {
        let json_str = r#"{
            "id": "1",
            "choices": [{
                "message": {
                    "content": "",
                    "reasoning_content": "Here is the full project analysis."
                },
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 2}
        }"#;
        let z: ZaiResponse = serde_json::from_str(json_str).unwrap();
        let r = convert_response(z).unwrap();
        assert!(r.tool_calls.is_empty());
        match &r.message.content {
            MessageContent::Text(t) => assert!(t.contains("full project analysis")),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn parses_response_content_as_openai_text_parts_array() {
        let json_str = r#"{
            "id": "1",
            "choices": [{
                "message": {
                    "content": [{"type":"text","text":"Line A"},{"type":"text","text":"Line B"}]
                },
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 2}
        }"#;
        let z: ZaiResponse = serde_json::from_str(json_str).unwrap();
        let r = convert_response(z).unwrap();
        match &r.message.content {
            MessageContent::Text(t) => {
                assert!(t.contains("Line A"));
                assert!(t.contains("Line B"));
            }
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn parses_response_tool_calls_ignores_reasoning_in_message_text() {
        let json_str = r#"{
            "id": "1",
            "choices": [{
                "message": {
                    "content": "short",
                    "reasoning_content": "LONG INTERNAL CHAIN",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "Bash", "arguments": "{}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 2}
        }"#;
        let z: ZaiResponse = serde_json::from_str(json_str).unwrap();
        let r = convert_response(z).unwrap();
        match &r.message.content {
            MessageContent::Text(t) => {
                assert_eq!(t, "short");
                assert!(!t.contains("INTERNAL"));
            }
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn parses_response_tool_calls_empty_content_falls_back_to_reasoning() {
        let json_str = r#"{
            "id": "1",
            "choices": [{
                "message": {
                    "content": "",
                    "reasoning_content": "Planner summary before tools.",
                    "tool_calls": [{
                        "id": "call_1",
                        "type": "function",
                        "function": {"name": "Bash", "arguments": "{}"}
                    }]
                },
                "finish_reason": "tool_calls"
            }],
            "usage": {"prompt_tokens": 1, "completion_tokens": 2}
        }"#;
        let z: ZaiResponse = serde_json::from_str(json_str).unwrap();
        let r = convert_response(z).unwrap();
        match &r.message.content {
            MessageContent::Text(t) => assert!(t.contains("Planner summary")),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn parses_response_strips_think_tags_into_reasoning_bucket() {
        let think_o = String::from_utf8(vec![60, 116, 104, 105, 110, 107, 62]).unwrap();
        let think_c = String::from_utf8(vec![60, 47, 116, 104, 105, 110, 107, 62]).unwrap();
        let content = format!("{}deep{}\n\nHello user.", think_o, think_c);
        let v = json!({
            "id": "1",
            "choices": [{
                "message": { "content": content },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 1, "completion_tokens": 2 }
        });
        let z: ZaiResponse = serde_json::from_value(v).unwrap();
        let r = convert_response(z).unwrap();
        match &r.message.content {
            MessageContent::Text(t) => {
                assert!(t.contains("Hello user"));
                assert!(t.contains("deep"));
                assert!(
                    !t.contains(think_o.as_str()),
                    "thinking tags should be stripped from merged text"
                );
            }
            _ => panic!("expected text"),
        }
    }
}
