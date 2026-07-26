//! OpenAI-compatible tool-call normalization: native JSON `tool_calls` only.
//!
//! NOTE: the legacy `function_call` migration chain (schema validation +
//! legacy parsing) is a test-covered reference implementation not yet wired
//! into production providers — hence the module-level allow.
#![allow(dead_code)]

use anycode_core::prelude::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

/// Result of normalizing one assistant turn.
#[derive(Debug, Clone)]
pub struct NormalizedAssistantOutput {
    pub tool_calls: Vec<ToolCall>,
    pub visible_content: String,
}

impl Default for NormalizedAssistantOutput {
    fn default() -> Self {
        Self {
            tool_calls: Vec::new(),
            visible_content: String::new(),
        }
    }
}

fn schema_by_name<'a>(tools: &'a [ToolSchema], name: &str) -> Option<&'a ToolSchema> {
    tools.iter().find(|t| t.name == name)
}

fn json_type_matches(value: &Value, ty: &str) -> bool {
    match ty {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "integer" => value.as_i64().is_some() || value.as_u64().is_some(),
        "boolean" => value.is_boolean(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn value_matches_schema(value: &Value, schema: &Value) -> bool {
    if let Some(ty) = schema.get("type").and_then(|t| t.as_str()) {
        if !json_type_matches(value, ty) {
            return false;
        }
    }
    if let Some(arr) = schema.get("enum").and_then(|e| e.as_array()) {
        if !arr.iter().any(|v| v == value) {
            return false;
        }
    }
    true
}

/// Validate a candidate tool invocation against the registered schema for this turn.
pub fn validate_tool_call_against_schema(name: &str, input: &Value, tools: &[ToolSchema]) -> bool {
    let Some(schema) = schema_by_name(tools, name) else {
        return false;
    };
    let params = &schema.input_schema;
    if !input.is_object() {
        return false;
    }
    let obj = input.as_object().unwrap();
    if let Some(required) = params.get("required").and_then(|r| r.as_array()) {
        for req in required {
            let Some(key) = req.as_str() else {
                continue;
            };
            if !obj.contains_key(key) {
                return false;
            }
        }
    }
    if let Some(props) = params.get("properties").and_then(|p| p.as_object()) {
        for (key, val) in obj {
            if let Some(prop_schema) = props.get(key) {
                if !value_matches_schema(val, prop_schema) {
                    return false;
                }
            }
        }
    }
    true
}

fn make_tool_call(name: String, input: Value, index: usize) -> ToolCall {
    ToolCall {
        id: format!("call_{index}_{}", Uuid::new_v4()),
        name,
        input,
    }
}

pub(crate) fn parse_arguments_value(raw: &str) -> Value {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return json!({});
    }
    if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
        return v;
    }
    // Truncated JSON from streaming: try to salvage a Bash-style {"command":"..."} prefix.
    if let Some(cmd) = salvage_command_from_truncated_json(trimmed) {
        return json!({ "command": cmd });
    }
    json!({ "raw": raw })
}

/// Recover `"command":"..."` from incomplete JSON when the closing quote/brace was cut off.
fn salvage_command_from_truncated_json(raw: &str) -> Option<String> {
    let key = "\"command\"";
    let idx = raw.find(key)?;
    let after = raw[idx + key.len()..].trim_start();
    let after = after.strip_prefix(':')?.trim_start();
    let after = after.strip_prefix('"')?;
    let mut out = String::new();
    let mut chars = after.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(n) = chars.next() {
                out.push(match n {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    other => other,
                });
            }
            continue;
        }
        if c == '"' {
            break;
        }
        out.push(c);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Parse native OpenAI `tool_calls` array from a message JSON value.
pub fn parse_native_tool_calls_from_message(message: &Value) -> Vec<ToolCall> {
    let mut out = Vec::new();
    let Some(arr) = message.get("tool_calls").and_then(|v| v.as_array()) else {
        return out;
    };
    for (i, part) in arr.iter().enumerate() {
        let id = part
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let id = if id.is_empty() {
            format!("call_{i}")
        } else {
            id
        };
        let f = part.get("function").or_else(|| part.get("function_call"));
        let Some(f) = f else {
            continue;
        };
        let name = f
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if name.is_empty() {
            continue;
        }
        let args = f
            .get("arguments")
            .map(|v| {
                if let Some(s) = v.as_str() {
                    parse_arguments_value(s)
                } else if v.is_object() {
                    v.clone()
                } else {
                    json!({})
                }
            })
            .unwrap_or_else(|| json!({}));
        out.push(ToolCall {
            id,
            name,
            input: args,
        });
    }
    out
}

fn parse_legacy_function_call(message: &Value) -> Option<ToolCall> {
    let fc = message.get("function_call")?;
    let name = fc.get("name")?.as_str()?.to_string();
    let args = fc
        .get("arguments")
        .map(|v| {
            if let Some(s) = v.as_str() {
                parse_arguments_value(s)
            } else {
                json!({})
            }
        })
        .unwrap_or_else(|| json!({}));
    Some(ToolCall {
        id: "call_legacy".to_string(),
        name,
        input: args,
    })
}

/// Normalize non-streaming assistant output (native tool_calls only).
pub fn normalize_assistant_output(
    native_tool_calls: Vec<ToolCall>,
    assistant_text: &str,
    _tools: &[ToolSchema],
) -> NormalizedAssistantOutput {
    NormalizedAssistantOutput {
        tool_calls: native_tool_calls,
        visible_content: assistant_text.to_string(),
    }
}

/// Full message JSON normalization (native + legacy function_call).
/// Currently only exercised in tests; kept as the reference implementation
/// for the legacy `function_call` migration path.
#[cfg_attr(not(test), allow(dead_code))]
pub fn normalize_from_openai_message(
    message: &Value,
    assistant_text: &str,
    tools: &[ToolSchema],
) -> NormalizedAssistantOutput {
    let mut native = parse_native_tool_calls_from_message(message);
    if native.is_empty() {
        if let Some(legacy) = parse_legacy_function_call(message) {
            native.push(legacy);
        }
    }
    normalize_assistant_output(native, assistant_text, tools)
}

/// Stateful OpenAI SSE stream parser (native structured tool_calls only).
pub struct OpenAiCompatStreamState {
    /// Tool schemas offered to the model (kept for future per-call validation).
    #[allow(dead_code)]
    pub tools: Vec<ToolSchema>,
    tool_builders: HashMap<u64, (Option<String>, Option<String>, String)>,
    emitted_structured_tools: bool,
    reasoning_acc: String,
}

impl OpenAiCompatStreamState {
    pub fn new(tools: Vec<ToolSchema>) -> Self {
        Self {
            tools,
            tool_builders: HashMap::new(),
            emitted_structured_tools: false,
            reasoning_acc: String::new(),
        }
    }

    fn flush_structured_tool_builders(&mut self) -> Vec<ToolCall> {
        let mut indices: Vec<u64> = self.tool_builders.keys().copied().collect();
        indices.sort_unstable();
        let mut out = Vec::new();
        for idx in indices {
            if let Some((id_o, name_o, args)) = self.tool_builders.remove(&idx) {
                let id = id_o.unwrap_or_else(|| format!("call_{idx}"));
                let name = name_o.unwrap_or_default();
                let input = if args.trim().is_empty() {
                    json!({})
                } else {
                    parse_arguments_value(&args)
                };
                out.push(ToolCall { id, name, input });
            }
        }
        if !out.is_empty() {
            self.emitted_structured_tools = true;
        }
        out
    }

    fn take_reasoning_event(&mut self) -> Option<StreamEvent> {
        let r = std::mem::take(&mut self.reasoning_acc);
        let t = r.trim();
        if t.is_empty() {
            None
        } else {
            Some(StreamEvent::Reasoning(t.to_string()))
        }
    }

    /// Process one SSE JSON chunk. Returns events to emit (deltas, tool calls, usage).
    pub fn process_chunk(&mut self, val: &Value) -> StreamChunkOutcome {
        let mut events = Vec::new();

        if let Some(usage) = val
            .get("usage")
            .and_then(crate::openai_compat_stream::openai_stream_usage_from_value)
        {
            events.push(StreamEvent::Usage(usage));
        }

        let choice = val
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.first());

        if let Some(delta) = choice.and_then(|c| c.get("delta")) {
            if let Some(arr) = delta.get("tool_calls").and_then(|x| x.as_array()) {
                for part in arr {
                    let index = part.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
                    let entry =
                        self.tool_builders
                            .entry(index)
                            .or_insert((None, None, String::new()));
                    if let Some(id) = part.get("id").and_then(|i| i.as_str()) {
                        entry.0 = Some(id.to_string());
                    }
                    if let Some(f) = part.get("function") {
                        if let Some(n) = f.get("name").and_then(|x| x.as_str()) {
                            entry.1.get_or_insert_with(|| n.to_string());
                        }
                        if let Some(a) = f.get("arguments").and_then(|x| x.as_str()) {
                            entry.2.push_str(a);
                        }
                    }
                }
            }

            // Keep reasoning separate from visible content (DeepSeek must echo reasoning_content).
            if let Some(r) = delta
                .get("reasoning_content")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
            {
                self.reasoning_acc.push_str(r);
            }

            if let Some(c) = delta
                .get("content")
                .and_then(|x| x.as_str())
                .filter(|s| !s.is_empty())
            {
                events.push(StreamEvent::Delta(c.to_string()));
            }
        }

        let finish = choice
            .and_then(|c| c.get("finish_reason"))
            .and_then(|r| r.as_str());

        if matches!(finish, Some("tool_calls") | Some("function_call"))
            && !self.tool_builders.is_empty()
        {
            for tc in self.flush_structured_tool_builders() {
                events.push(StreamEvent::ToolCall(tc));
            }
        }

        if finish.is_some() && !self.tool_builders.is_empty() {
            for tc in self.flush_structured_tool_builders() {
                events.push(StreamEvent::ToolCall(tc));
            }
        }

        if finish.is_some() {
            if let Some(ev) = self.take_reasoning_event() {
                events.push(ev);
            }
        }

        StreamChunkOutcome { events }
    }

    /// Flush at stream EOF when no explicit finish_reason arrived.
    pub fn finish(&mut self) -> Vec<StreamEvent> {
        let mut events = Vec::new();
        if !self.tool_builders.is_empty() {
            for tc in self.flush_structured_tool_builders() {
                events.push(StreamEvent::ToolCall(tc));
            }
        }
        if let Some(ev) = self.take_reasoning_event() {
            events.push(ev);
        }
        events
    }
}

pub struct StreamChunkOutcome {
    pub events: Vec<StreamEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weather_schema() -> ToolSchema {
        ToolSchema {
            name: "get_weather".to_string(),
            description: "Get weather".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": { "city": { "type": "string" } },
                "required": ["city"]
            }),
        }
    }

    #[test]
    fn validates_required_fields() {
        let tools = vec![weather_schema()];
        assert!(validate_tool_call_against_schema(
            "get_weather",
            &json!({"city": "Beijing"}),
            &tools
        ));
        assert!(!validate_tool_call_against_schema(
            "get_weather",
            &json!({}),
            &tools
        ));
        assert!(!validate_tool_call_against_schema(
            "get_weather",
            &json!({"city": 1}),
            &tools
        ));
    }

    #[test]
    fn native_tool_calls_take_priority() {
        let native = vec![ToolCall {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            input: json!({"city": "Shanghai"}),
        }];
        let text = "Here is the weather update.";
        let out = normalize_assistant_output(native.clone(), text, &[weather_schema()]);
        assert_eq!(out.tool_calls.len(), native.len());
        assert_eq!(out.tool_calls[0].name, native[0].name);
        assert_eq!(out.tool_calls[0].input, native[0].input);
        assert_eq!(out.visible_content, text);
    }

    #[test]
    fn parse_native_from_message_json() {
        let msg = json!({
            "role": "assistant",
            "content": null,
            "tool_calls": [{
                "id": "call_abc",
                "type": "function",
                "function": { "name": "get_weather", "arguments": "{\"city\":\"Beijing\"}" }
            }]
        });
        let parsed = parse_native_tool_calls_from_message(&msg);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].id, "call_abc");
        assert_eq!(parsed[0].input["city"], "Beijing");
    }

    #[test]
    fn stream_state_emits_structured_tool_calls_on_finish_reason() {
        let mut state = OpenAiCompatStreamState::new(vec![weather_schema()]);
        let chunk = json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "call_1",
                        "function": { "name": "get_weather", "arguments": "{\"city\":\"" }
                    }]
                },
                "finish_reason": null
            }]
        });
        let out1 = state.process_chunk(&chunk);
        assert!(out1.events.is_empty());

        let chunk2 = json!({
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "function": { "arguments": "Beijing\"}" }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let out2 = state.process_chunk(&chunk2);
        assert!(out2
            .events
            .iter()
            .any(|e| matches!(e, StreamEvent::ToolCall(_))));
    }

    #[test]
    fn stream_state_emits_content_deltas_directly() {
        let mut state = OpenAiCompatStreamState::new(vec![weather_schema()]);
        let chunk = json!({
            "choices": [{
                "delta": { "content": "Hello" },
                "finish_reason": null
            }]
        });
        let out = state.process_chunk(&chunk);
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, StreamEvent::Delta(s) if s == "Hello")));
    }

    #[test]
    fn stream_keeps_reasoning_out_of_content_deltas() {
        let mut state = OpenAiCompatStreamState::new(vec![weather_schema()]);
        let chunk = json!({
            "choices": [{
                "delta": {
                    "reasoning_content": "plan A",
                    "tool_calls": [{
                        "index": 0,
                        "id": "c1",
                        "function": { "name": "Bash", "arguments": "{\"command\":\"ls\"}" }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });
        let out = state.process_chunk(&chunk);
        assert!(!out
            .events
            .iter()
            .any(|e| matches!(e, StreamEvent::Delta(_))));
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, StreamEvent::ToolCall(_))));
        assert!(out
            .events
            .iter()
            .any(|e| matches!(e, StreamEvent::Reasoning(s) if s == "plan A")));
    }

    #[test]
    fn salvage_truncated_bash_command_json() {
        let v = parse_arguments_value("{\"command\": \"cd /Users/qingjiu");
        assert_eq!(
            v.get("command").and_then(|x| x.as_str()),
            Some("cd /Users/qingjiu")
        );
        assert!(v.get("raw").is_none());
    }
}
