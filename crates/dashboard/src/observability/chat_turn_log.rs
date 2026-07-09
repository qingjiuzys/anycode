//! Canonical chat turn event log: persist, replay, and hydrate transcript blocks.

use crate::db::DashboardDb;
use crate::observability::chat_events::{
    live_assistant_block_id, live_tool_block_id, live_tool_key,
};
use crate::schema::{ChatStreamEvent, ChatTurnEventRecord, TranscriptBlock};
use anyhow::Result;
use chrono::Utc;
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn persist_and_enrich(
    db: &DashboardDb,
    mut evt: ChatStreamEvent,
    conversation_turn_id: u32,
) -> Result<ChatStreamEvent> {
    let record = persist_chat_event(db, &evt, conversation_turn_id).await?;
    enrich_stream_event(&mut evt, &record);
    crate::session_transcript::invalidate_transcript_cache(&evt.session_id);
    Ok(evt)
}

pub async fn persist_chat_event(
    db: &DashboardDb,
    evt: &ChatStreamEvent,
    conversation_turn_id: u32,
) -> Result<ChatTurnEventRecord> {
    let body = evt
        .text
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| evt.block.as_ref().map(|b| b.body.clone()))
        .unwrap_or_default();
    let block_json = evt.block.as_ref().map(serde_json::to_string).transpose()?;
    db.append_chat_turn_event(
        &evt.session_id,
        &evt.project_id,
        conversation_turn_id,
        evt.turn,
        &evt.kind,
        evt.tool_key.as_deref(),
        evt.tool_name.as_deref(),
        &body,
        block_json.as_deref(),
        &evt.payload,
        &evt.at,
    )
    .await
}

pub fn enrich_stream_event(evt: &mut ChatStreamEvent, record: &ChatTurnEventRecord) {
    evt.seq = Some(record.seq);
    evt.conversation_turn_id = Some(record.conversation_turn_id);
    evt.event_id = Some(record.id.clone());
}

#[must_use]
pub fn record_to_stream_event(record: &ChatTurnEventRecord) -> ChatStreamEvent {
    ChatStreamEvent {
        session_id: record.session_id.clone(),
        project_id: record.project_id.clone(),
        kind: record.kind.clone(),
        turn: record.agent_turn,
        conversation_turn_id: Some(record.conversation_turn_id),
        seq: Some(record.seq),
        event_id: Some(record.id.clone()),
        tool_key: record.tool_key.clone(),
        tool_name: record.tool_name.clone(),
        text: if record.body.is_empty() {
            None
        } else {
            Some(record.body.clone())
        },
        block: record
            .block_json
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok()),
        payload: record.payload.clone(),
        at: record.occurred_at.clone(),
    }
}

#[must_use]
pub fn user_message_event(
    session_id: &str,
    project_id: &str,
    conversation_turn_id: u32,
    prompt: &str,
) -> ChatStreamEvent {
    let at = Utc::now().to_rfc3339();
    let block_id = format!("user:u{conversation_turn_id}:{}", Uuid::new_v4().simple());
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "user_message".into(),
        turn: None,
        conversation_turn_id: Some(conversation_turn_id),
        seq: None,
        event_id: None,
        tool_key: None,
        tool_name: None,
        text: Some(prompt.to_string()),
        block: Some(TranscriptBlock {
            id: block_id,
            block_type: "user_message".into(),
            at: at.clone(),
            title: "You".into(),
            body: prompt.to_string(),
            meta: json!({
                "user_turn_id": conversation_turn_id.to_string(),
                "conversation_turn_id": conversation_turn_id,
            }),
            collapsible: false,
            default_collapsed: false,
            event_id: None,
        }),
        payload: json!({
            "user_turn_id": conversation_turn_id,
            "conversation_turn_id": conversation_turn_id,
        }),
        at,
    }
}

#[must_use]
pub fn records_to_transcript_blocks(records: &[ChatTurnEventRecord]) -> Vec<TranscriptBlock> {
    let mut blocks: Vec<TranscriptBlock> = Vec::new();
    for record in records {
        let evt = record_to_stream_event(record);
        blocks = apply_chat_stream_event(blocks, &evt);
    }
    blocks
}

fn upsert_block(blocks: Vec<TranscriptBlock>, block: TranscriptBlock) -> Vec<TranscriptBlock> {
    if let Some(idx) = blocks.iter().position(|b| b.id == block.id) {
        let mut next = blocks;
        next[idx] = merge_block(&next[idx], &block);
        return next;
    }
    let mut next = blocks;
    next.push(block);
    next
}

fn merge_block(prev: &TranscriptBlock, incoming: &TranscriptBlock) -> TranscriptBlock {
    let live = incoming
        .meta
        .get("live")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if live || incoming.body.len() >= prev.body.len() {
        TranscriptBlock {
            id: prev.id.clone(),
            block_type: incoming.block_type.clone(),
            at: incoming.at.clone(),
            title: incoming.title.clone(),
            body: incoming.body.clone(),
            meta: merge_json(&prev.meta, &incoming.meta),
            collapsible: incoming.collapsible,
            default_collapsed: incoming.default_collapsed,
            event_id: incoming.event_id.clone().or_else(|| prev.event_id.clone()),
        }
    } else {
        prev.clone()
    }
}

fn merge_json(a: &Value, b: &Value) -> Value {
    match (a, b) {
        (Value::Object(a_map), Value::Object(b_map)) => {
            let mut out = a_map.clone();
            for (k, v) in b_map {
                out.insert(k.clone(), v.clone());
            }
            Value::Object(out)
        }
        (_, b) => b.clone(),
    }
}

fn apply_chat_stream_event(
    blocks: Vec<TranscriptBlock>,
    evt: &ChatStreamEvent,
) -> Vec<TranscriptBlock> {
    match evt.kind.as_str() {
        "user_message" => {
            if let Some(block) = evt.block.as_ref() {
                upsert_block(blocks, block.clone())
            } else {
                blocks
            }
        }
        "assistant_delta" => {
            let turn = evt.turn.unwrap_or(1);
            let user_turn_id = evt.conversation_turn_id.or_else(|| {
                evt.payload
                    .get("user_turn_id")
                    .and_then(|v| v.as_u64())
                    .map(|n| n as u32)
            });
            let id = evt.block.as_ref().map(|b| b.id.clone()).unwrap_or_else(|| {
                user_turn_id
                    .map(|u| live_assistant_block_id(u, turn))
                    .unwrap_or_else(|| format!("assistant-live:{turn}"))
            });
            let existing = blocks.iter().find(|b| b.id == id);
            let body = evt
                .block
                .as_ref()
                .map(|b| b.body.clone())
                .unwrap_or_else(|| {
                    format!(
                        "{}{}",
                        existing.map(|b| b.body.as_str()).unwrap_or(""),
                        evt.text.as_deref().unwrap_or("")
                    )
                });
            let mut meta = existing.map(|b| b.meta.clone()).unwrap_or(json!({}));
            if let Value::Object(ref mut map) = meta {
                map.insert("live".into(), json!(true));
                map.insert("turn".into(), json!(turn));
                if let Some(u) = user_turn_id {
                    map.insert("user_turn_id".into(), json!(u.to_string()));
                    map.insert("conversation_turn_id".into(), json!(u));
                }
            }
            let block = TranscriptBlock {
                id,
                block_type: "assistant_message".into(),
                at: evt.at.clone(),
                title: evt
                    .block
                    .as_ref()
                    .map(|b| b.title.clone())
                    .unwrap_or_else(|| format!("Assistant (turn {turn})")),
                body,
                meta,
                collapsible: false,
                default_collapsed: false,
                event_id: existing
                    .and_then(|b| b.event_id.clone())
                    .or_else(|| evt.block.as_ref().and_then(|b| b.event_id.clone())),
            };
            upsert_block(blocks, block)
        }
        "assistant_done" => {
            if let Some(block) = evt.block.as_ref() {
                let mut merged = block.clone();
                if let Value::Object(ref mut map) = merged.meta {
                    map.insert("live".into(), json!(false));
                }
                upsert_block(blocks, merged)
            } else {
                blocks
            }
        }
        "tool_start" | "tool_result" | "tool_progress" | "llm_start" | "session_error" => {
            if let Some(block) = evt.block.as_ref() {
                upsert_block(blocks, block.clone())
            } else {
                blocks
            }
        }
        "turn_done" => blocks,
        _ => blocks,
    }
}

#[must_use]
pub fn live_tool_progress_event(
    session_id: &str,
    project_id: &str,
    user_turn_id: u32,
    turn: u32,
    idx: u32,
    name: &str,
    elapsed_ms: u64,
) -> ChatStreamEvent {
    let at = Utc::now().to_rfc3339();
    let tool_key = live_tool_key(user_turn_id, turn, idx);
    ChatStreamEvent {
        session_id: session_id.to_string(),
        project_id: project_id.to_string(),
        kind: "tool_progress".into(),
        turn: Some(turn),
        conversation_turn_id: Some(user_turn_id),
        seq: None,
        event_id: None,
        tool_key: Some(tool_key.clone()),
        tool_name: Some(name.to_string()),
        text: None,
        block: Some(TranscriptBlock {
            id: live_tool_block_id(user_turn_id, turn, idx, "call"),
            block_type: "tool_call".into(),
            at: at.clone(),
            title: format!("{name} running"),
            body: String::new(),
            meta: json!({
                "phase": "start",
                "live": true,
                "turn": turn,
                "idx": idx,
                "name": name,
                "tool_key": tool_key,
                "elapsed_ms": elapsed_ms.to_string(),
                "user_turn_id": user_turn_id.to_string(),
                "conversation_turn_id": user_turn_id,
            }),
            collapsible: true,
            default_collapsed: true,
            event_id: None,
        }),
        payload: json!({
            "turn": turn,
            "idx": idx,
            "name": name,
            "elapsed_ms": elapsed_ms,
            "user_turn_id": user_turn_id,
        }),
        at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ProjectEvent;

    #[test]
    fn records_apply_in_seq_order() {
        let user = user_message_event("s1", "p1", 1, "hello");
        let mut records = Vec::new();
        for (seq, evt) in [(1, user)].into_iter() {
            records.push(ChatTurnEventRecord {
                id: format!("e{seq}"),
                session_id: "s1".into(),
                project_id: "p1".into(),
                conversation_turn_id: 1,
                agent_turn: None,
                seq,
                kind: evt.kind.clone(),
                tool_key: None,
                tool_name: None,
                body: evt.text.clone().unwrap_or_default(),
                block_json: evt
                    .block
                    .as_ref()
                    .map(|b| serde_json::to_string(b).unwrap()),
                payload: evt.payload.clone(),
                occurred_at: evt.at.clone(),
            });
        }
        let blocks = records_to_transcript_blocks(&records);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].block_type, "user_message");
        assert_eq!(blocks[0].body, "hello");
    }

    #[test]
    fn user_message_has_conversation_turn_id() {
        let evt = user_message_event("s1", "p1", 3, "test");
        assert_eq!(evt.conversation_turn_id, Some(3));
        assert!(evt.block.is_some());
    }

    #[allow(dead_code)]
    fn _project_event_stub() -> ProjectEvent {
        ProjectEvent {
            id: "e".into(),
            project_id: "p".into(),
            session_id: Some("s".into()),
            task_id: None,
            agent_id: None,
            event_type: "user_prompt".into(),
            severity: "info".into(),
            title: "You".into(),
            body: "hi".into(),
            payload: json!({}),
            occurred_at: "2026-01-01T00:00:00Z".into(),
        }
    }
}
