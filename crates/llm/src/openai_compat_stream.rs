//! OpenAI Chat Completions SSE chunk → [`StreamEvent`]（z.ai / gateway / OpenAI 共用）。

use crate::tool_call_normalizer::OpenAiCompatStreamState;
use anycode_core::prelude::*;
use serde_json::Value;
use tokio::sync::mpsc;

pub fn openai_stream_usage_from_value(v: &Value) -> Option<Usage> {
    Some(Usage {
        input_tokens: v.get("prompt_tokens")?.as_u64()? as u32,
        output_tokens: v.get("completion_tokens")?.as_u64()? as u32,
        cache_creation_tokens: None,
        cache_read_tokens: None,
    })
}

/// Process one SSE JSON chunk with tool-aware state. `true` = stop (tx closed).
pub async fn emit_openai_sse_with_state(
    val: &Value,
    tx: &mpsc::Sender<StreamEvent>,
    state: &mut OpenAiCompatStreamState,
) -> bool {
    let outcome = state.process_chunk(val);
    for ev in outcome.events {
        if tx.send(ev).await.is_err() {
            return true;
        }
    }
    false
}

/// Flush pending tool calls / textual fallback at stream end. `true` = stop (tx closed).
pub async fn flush_openai_sse_state(
    tx: &mpsc::Sender<StreamEvent>,
    state: &mut OpenAiCompatStreamState,
) -> bool {
    for ev in state.finish() {
        if tx.send(ev).await.is_err() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool_call_normalizer::OpenAiCompatStreamState;
    use anycode_core::prelude::*;
    use serde_json::json;
    use tokio::sync::mpsc;

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

    #[tokio::test]
    async fn stream_emits_usage_and_tool_call() {
        let (tx, mut rx) = mpsc::channel(8);
        let mut state = OpenAiCompatStreamState::new(vec![weather_schema()]);
        let chunk = json!({
            "usage": { "prompt_tokens": 1, "completion_tokens": 2 },
            "choices": [{
                "delta": {
                    "tool_calls": [{
                        "index": 0,
                        "id": "c1",
                        "function": { "name": "get_weather", "arguments": "{\"city\":\"BJ\"}" }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });
        assert!(!emit_openai_sse_with_state(&chunk, &tx, &mut state).await);
        drop(tx);
        let mut got_usage = false;
        let mut got_tool = false;
        while let Some(ev) = rx.recv().await {
            match ev {
                StreamEvent::Usage(_) => got_usage = true,
                StreamEvent::ToolCall(tc) => {
                    got_tool = true;
                    assert_eq!(tc.name, "get_weather");
                }
                _ => {}
            }
        }
        assert!(got_usage);
        assert!(got_tool);
    }
}
