//! Tail web-chat `output.log` and publish low-latency [`ChatStreamEvent`]s.

use crate::events::EventBus;
use crate::observability::chat_events::{assistant_delta_event, chat_event_from_parsed_line};
use crate::observability::log_parser::{parse_line, parse_prose_sections};
use crate::schema::ProjectEvent;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

type TailRegistry = Arc<Mutex<HashMap<String, CancellationToken>>>;

#[derive(Clone, Default)]
pub struct WebChatTailHub {
    registry: TailRegistry,
}

impl WebChatTailHub {
    pub fn ensure_tail(
        &self,
        events: Arc<EventBus>,
        session_id: &str,
        project_id: &str,
        log_path: &Path,
    ) {
        let session_id = session_id.to_string();
        let project_id = project_id.to_string();
        let log_path = log_path.to_path_buf();
        let registry = Arc::clone(&self.registry);
        let events = Arc::clone(&events);
        tokio::spawn(async move {
            let mut guard = registry.lock().await;
            if guard.contains_key(&session_id) {
                return;
            }
            let cancel = CancellationToken::new();
            guard.insert(session_id.clone(), cancel.clone());
            drop(guard);
            let session_key = session_id.clone();
            run_tail(events, session_id, project_id, log_path, cancel).await;
            registry.lock().await.remove(&session_key);
        });
    }

    pub async fn stop_tail(&self, session_id: &str) {
        if let Some(token) = self.registry.lock().await.remove(session_id) {
            token.cancel();
        }
    }
}

async fn run_tail(
    events: Arc<EventBus>,
    session_id: String,
    project_id: String,
    log_path: PathBuf,
    cancel: CancellationToken,
) {
    let mut offset: u64 = 0;
    let mut carry = String::new();
    let mut assistant_turn: u32 = 0;
    let mut assistant_text = String::new();
    let mut in_assistant_final = false;

    loop {
        if cancel.is_cancelled() {
            break;
        }
        tokio::select! {
            _ = cancel.cancelled() => break,
            _ = tokio::time::sleep(Duration::from_millis(180)) => {}
        }

        let chunk = match read_from_offset(&log_path, offset) {
            Ok(Some((next, text))) => {
                offset = next;
                text
            }
            Ok(None) => continue,
            Err(_) => continue,
        };

        carry.push_str(&chunk);
        while let Some(pos) = carry.find('\n') {
            let line = carry[..pos].to_string();
            carry = carry[pos + 1..].to_string();
            if let Some(parsed) = parse_line(&line) {
                if parsed.event_type == "turn_start" {
                    if let Some(t) = parsed.payload.get("turn").and_then(|v| v.as_str()) {
                        assistant_turn = t.parse().unwrap_or(assistant_turn);
                    }
                    assistant_text.clear();
                    in_assistant_final = false;
                }
                if let Some(chat_evt) =
                    chat_event_from_parsed_line(&session_id, &project_id, &parsed)
                {
                    events.publish_chat(chat_evt);
                }
            }

            let trimmed = line.trim();
            if trimmed == "== assistant_final ==" {
                in_assistant_final = true;
                assistant_text.clear();
                continue;
            }
            if in_assistant_final {
                if trimmed.starts_with("== ") && trimmed.ends_with(" ==") {
                    in_assistant_final = false;
                    continue;
                }
                if !trimmed.is_empty() {
                    let prev_len = assistant_text.chars().count();
                    if !assistant_text.is_empty() {
                        assistant_text.push('\n');
                    }
                    assistant_text.push_str(trimmed);
                    let delta = trimmed.to_string();
                    if assistant_turn == 0 {
                        assistant_turn = 1;
                    }
                    events.publish_chat(assistant_delta_event(
                        &session_id,
                        &project_id,
                        assistant_turn,
                        &delta,
                        &assistant_text,
                    ));
                    let _ = prev_len;
                }
            }
        }

        if let Ok(content) = std::fs::read_to_string(&log_path) {
            for (turn, prose) in parse_prose_sections(&content) {
                let turn = turn as u32;
                if prose.len() > assistant_text.len() && turn >= assistant_turn {
                    let delta = prose[assistant_text.len()..].to_string();
                    if !delta.is_empty() {
                        assistant_turn = turn;
                        assistant_text = prose.clone();
                        events.publish_chat(assistant_delta_event(
                            &session_id,
                            &project_id,
                            assistant_turn,
                            &delta,
                            &assistant_text,
                        ));
                    }
                }
            }
        }
    }
}

fn read_from_offset(path: &Path, mut offset: u64) -> std::io::Result<Option<(u64, String)>> {
    if !path.is_file() {
        return Ok(None);
    }
    let mut file = std::fs::File::open(path)?;
    let len = file.metadata()?.len();
    if offset > len {
        offset = 0;
    }
    if offset == len {
        return Ok(None);
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut buf = vec![0u8; (len - offset) as usize];
    let read = file.read(&mut buf)?;
    if read == 0 {
        return Ok(None);
    }
    buf.truncate(read);
    let text = String::from_utf8_lossy(&buf).into_owned();
    Ok(Some((offset + read as u64, text)))
}

/// Publish a persisted project event and its chat mapping (for handlers that insert directly).
pub fn publish_project_chat_event(events: &EventBus, evt: &ProjectEvent) {
    events.publish(evt.clone());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::ProjectEvent;
    use serde_json::json;
    use tempfile::NamedTempFile;

    #[test]
    fn tail_reads_appended_lines() {
        let file = NamedTempFile::new().unwrap();
        std::fs::write(file.path(), "[turn_start] turn=1\n").unwrap();
        let (off, text) = read_from_offset(file.path(), 0).unwrap().unwrap();
        assert!(text.contains("turn_start"));
        std::fs::write(
            file.path(),
            "[turn_start] turn=1\n[tool_call_start] turn=1 idx=1 name=Bash\n",
        )
        .unwrap();
        let (off2, text2) = read_from_offset(file.path(), off).unwrap().unwrap();
        assert!(text2.contains("tool_call_start"));
        assert!(off2 > off);
    }

    #[test]
    fn publish_project_chat_event_maps_tool_start() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe_chat();
        let evt = ProjectEvent {
            id: "e1".into(),
            project_id: "p1".into(),
            session_id: Some("s1".into()),
            task_id: None,
            agent_id: None,
            event_type: "tool_call_start".into(),
            severity: "info".into(),
            title: "Bash".into(),
            body: "ls".into(),
            payload: json!({ "turn": "1", "idx": "1", "name": "Bash" }),
            occurred_at: "2026-01-01T00:00:00Z".into(),
        };
        publish_project_chat_event(&bus, &evt);
        let chat = rx.try_recv().expect("chat event");
        assert_eq!(chat.kind, "tool_start");
    }
}
