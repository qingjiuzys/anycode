//! Anthropic Messages API 流式 SSE：`data:` 行为 JSON 事件（非 [`StreamEvent`] 裸反序列化）。

use anycode_core::prelude::*;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default)]
struct ToolBlockAcc {
    id: String,
    name: String,
    input_json: String,
}

/// 将单条 `data:` JSON 解析为 0..n 个 [`StreamEvent`]（维护 tool 块累加状态）。
#[derive(Default)]
pub struct AnthropicSseStreamState {
    tool_blocks: HashMap<usize, ToolBlockAcc>,
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    cache_creation_tokens: Option<u32>,
    cache_read_tokens: Option<u32>,
}

impl AnthropicSseStreamState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_json_str(&mut self, data: &str) -> Result<Vec<StreamEvent>, serde_json::Error> {
        let v: Value = serde_json::from_str(data)?;
        Ok(self.push_value(&v))
    }

    fn push_value(&mut self, value: &Value) -> Vec<StreamEvent> {
        let mut out = Vec::new();
        let Some(ev_type) = value.get("type").and_then(|t| t.as_str()) else {
            return out;
        };

        match ev_type {
            "content_block_start" => {
                let index = value.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                if let Some(block) = value.get("content_block") {
                    if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                        let id = block
                            .get("id")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();
                        let name = block
                            .get("name")
                            .and_then(|x| x.as_str())
                            .unwrap_or("")
                            .to_string();
                        self.tool_blocks.insert(
                            index,
                            ToolBlockAcc {
                                id,
                                name,
                                input_json: String::new(),
                            },
                        );
                    }
                }
            }
            "content_block_delta" => {
                let index = value.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                let Some(delta) = value.get("delta") else {
                    return out;
                };
                match delta.get("type").and_then(|t| t.as_str()) {
                    Some("text_delta") => {
                        if let Some(text) = delta.get("text").and_then(|x| x.as_str()) {
                            if !text.is_empty() {
                                out.push(StreamEvent::Delta(text.to_string()));
                            }
                        }
                    }
                    Some("input_json_delta") => {
                        if let Some(pj) = delta.get("partial_json").and_then(|x| x.as_str()) {
                            if let Some(acc) = self.tool_blocks.get_mut(&index) {
                                acc.input_json.push_str(pj);
                            }
                        }
                    }
                    _ => {}
                }
            }
            "content_block_stop" => {
                let index = value.get("index").and_then(|i| i.as_u64()).unwrap_or(0) as usize;
                if let Some(acc) = self.tool_blocks.remove(&index) {
                    if !acc.id.is_empty() && !acc.name.is_empty() {
                        let input: Value = if acc.input_json.trim().is_empty() {
                            serde_json::json!({})
                        } else {
                            serde_json::from_str(&acc.input_json).unwrap_or_else(|_| {
                                serde_json::Value::String(acc.input_json.clone())
                            })
                        };
                        out.push(StreamEvent::ToolCall(ToolCall {
                            id: acc.id,
                            name: acc.name,
                            input,
                        }));
                    }
                }
            }
            "message_stop" | "ping" => {}
            "message_start" | "message_delta" => {
                if let Some(usage_ev) = self.extract_usage_event(value) {
                    out.push(usage_ev);
                }
            }
            _ => {}
        }

        out
    }

    fn extract_usage_event(&mut self, value: &Value) -> Option<StreamEvent> {
        let usage = value
            .get("usage")
            .or_else(|| value.get("message").and_then(|m| m.get("usage")))?;
        let mut changed = false;

        let set_u32 =
            |slot: &mut Option<u32>, key: &str, source: &Value, changed_flag: &mut bool| {
                if let Some(v) = source.get(key).and_then(|x| x.as_u64()) {
                    let v = v as u32;
                    if *slot != Some(v) {
                        *slot = Some(v);
                        *changed_flag = true;
                    }
                }
            };

        set_u32(&mut self.input_tokens, "input_tokens", usage, &mut changed);
        set_u32(
            &mut self.output_tokens,
            "output_tokens",
            usage,
            &mut changed,
        );
        set_u32(
            &mut self.cache_creation_tokens,
            "cache_creation_input_tokens",
            usage,
            &mut changed,
        );
        set_u32(
            &mut self.cache_read_tokens,
            "cache_read_input_tokens",
            usage,
            &mut changed,
        );

        if !changed {
            return None;
        }

        Some(StreamEvent::Usage(Usage {
            input_tokens: self.input_tokens.unwrap_or(0),
            output_tokens: self.output_tokens.unwrap_or(0),
            cache_creation_tokens: self.cache_creation_tokens,
            cache_read_tokens: self.cache_read_tokens,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_delta_emits() {
        let mut s = AnthropicSseStreamState::new();
        let ev = serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": { "type": "text_delta", "text": "Hi" }
        });
        let r = s.push_value(&ev);
        assert!(matches!(&r[..], [StreamEvent::Delta(t)] if t == "Hi"));
    }

    #[test]
    fn tool_use_streams_and_stops() {
        let mut s = AnthropicSseStreamState::new();
        s.push_value(&serde_json::json!({
            "type": "content_block_start",
            "index": 1,
            "content_block": { "type": "tool_use", "id": "tu1", "name": "Echo", "input": {} }
        }));
        s.push_value(&serde_json::json!({
            "type": "content_block_delta",
            "index": 1,
            "delta": { "type": "input_json_delta", "partial_json": "{\"a\":1}" }
        }));
        let r = s.push_value(&serde_json::json!({
            "type": "content_block_stop",
            "index": 1
        }));
        assert_eq!(r.len(), 1);
        match &r[0] {
            StreamEvent::ToolCall(tc) => {
                assert_eq!(tc.id, "tu1");
                assert_eq!(tc.name, "Echo");
                assert_eq!(tc.input, serde_json::json!({"a":1}));
            }
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn usage_event_emits_from_message_start_and_delta() {
        let mut s = AnthropicSseStreamState::new();
        let r1 = s.push_value(&serde_json::json!({
            "type": "message_start",
            "message": {
                "usage": {
                    "input_tokens": 123,
                    "cache_creation_input_tokens": 11
                }
            }
        }));
        assert!(matches!(
            &r1[..],
            [StreamEvent::Usage(Usage {
                input_tokens: 123,
                output_tokens: 0,
                cache_creation_tokens: Some(11),
                cache_read_tokens: None
            })]
        ));

        let r2 = s.push_value(&serde_json::json!({
            "type": "message_delta",
            "usage": {
                "output_tokens": 9
            }
        }));
        assert!(matches!(
            &r2[..],
            [StreamEvent::Usage(Usage {
                input_tokens: 123,
                output_tokens: 9,
                ..
            })]
        ));
    }
}
