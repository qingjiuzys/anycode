//! OpenAI Chat Completions SSE chunk → [`StreamEvent`]（z.ai / gateway / OpenAI 共用）。

use anycode_core::prelude::*;
use serde_json::{json, Value};
use std::collections::HashMap;
use tokio::sync::mpsc;

pub fn openai_stream_usage_from_value(v: &Value) -> Option<Usage> {
    Some(Usage {
        input_tokens: v.get("prompt_tokens")?.as_u64()? as u32,
        output_tokens: v.get("completion_tokens")?.as_u64()? as u32,
        cache_creation_tokens: None,
        cache_read_tokens: None,
    })
}

/// `true` 表示应停止（`tx` 已关闭）。
pub async fn emit_openai_sse_json_chunk(
    val: &Value,
    tx: &mpsc::Sender<StreamEvent>,
    tool_builders: &mut HashMap<u64, (Option<String>, Option<String>, String)>,
) -> bool {
    if let Some(usage) = val.get("usage").and_then(openai_stream_usage_from_value) {
        if tx.send(StreamEvent::Usage(usage)).await.is_err() {
            return true;
        }
    }
    let choice = val
        .get("choices")
        .and_then(|c| c.as_array())
        .and_then(|a| a.first());

    if let Some(delta) = choice.and_then(|c| c.get("delta")) {
        if let Some(c) = delta
            .get("content")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .or_else(|| delta.get("reasoning_content").and_then(|x| x.as_str()))
        {
            if tx.send(StreamEvent::Delta(c.to_string())).await.is_err() {
                return true;
            }
        }
        if let Some(arr) = delta.get("tool_calls").and_then(|x| x.as_array()) {
            for part in arr {
                let index = part.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
                let entry = tool_builders
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
    }

    if let Some(reason) = choice
        .and_then(|c| c.get("finish_reason"))
        .and_then(|r| r.as_str())
    {
        if reason == "tool_calls" {
            let mut indices: Vec<u64> = tool_builders.keys().copied().collect();
            indices.sort_unstable();
            for i in indices {
                if let Some((id_o, name_o, args)) = tool_builders.remove(&i) {
                    let id = id_o.unwrap_or_else(|| format!("call_{i}"));
                    let name = name_o.unwrap_or_default();
                    let input: Value = if args.trim().is_empty() {
                        json!({})
                    } else {
                        serde_json::from_str(&args).unwrap_or_else(|_| json!({ "raw": args }))
                    };
                    let tc = ToolCall { id, name, input };
                    if tx.send(StreamEvent::ToolCall(tc)).await.is_err() {
                        return true;
                    }
                }
            }
        }
    }
    false
}
