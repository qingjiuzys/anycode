//! Append-only JSONL session event chain for crash-safe resume.

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use uuid::Uuid;

const FLUSH_DEBOUNCE: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionChainEventKind {
    UserMessage,
    AssistantMessage,
    ToolCallStart,
    ToolCallEnd,
    ToolResult,
    ApprovalPending,
    ApprovalResolved,
    CompactBoundary,
    SnapshotBoundary,
    SyntheticToolResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SessionChainEvent {
    pub id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_id: Option<String>,
    pub kind: SessionChainEventKind,
    #[serde(default)]
    pub payload: serde_json::Value,
    pub ts: String,
}

pub(crate) fn chain_path_for_session(session_id: Uuid, sessions_root: &Path) -> PathBuf {
    sessions_root.join(format!("{session_id}.jsonl"))
}

pub(crate) struct SessionEventChain {
    path: PathBuf,
    seen: HashSet<Uuid>,
    pending: Vec<SessionChainEvent>,
    last_flush: Instant,
}

impl SessionEventChain {
    pub(crate) fn open(session_id: Uuid, sessions_root: &Path) -> anyhow::Result<Self> {
        std::fs::create_dir_all(sessions_root)
            .with_context(|| format!("create {}", sessions_root.display()))?;
        let path = chain_path_for_session(session_id, sessions_root);
        let mut seen = HashSet::new();
        if path.is_file() {
            let f = File::open(&path).with_context(|| format!("open {}", path.display()))?;
            for line in BufReader::new(f).lines() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                if let Ok(evt) = serde_json::from_str::<SessionChainEvent>(&line) {
                    seen.insert(evt.id);
                }
            }
        }
        Ok(Self {
            path,
            seen,
            pending: Vec::new(),
            last_flush: Instant::now(),
        })
    }

    pub(crate) fn append(&mut self, evt: SessionChainEvent) -> anyhow::Result<()> {
        if !self.seen.insert(evt.id) {
            return Ok(());
        }
        self.pending.push(evt);
        if self.last_flush.elapsed() >= FLUSH_DEBOUNCE {
            self.flush()?;
        }
        Ok(())
    }

    pub(crate) fn flush(&mut self) -> anyhow::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .with_context(|| format!("append {}", self.path.display()))?;
        for evt in self.pending.drain(..) {
            let line = serde_json::to_string(&evt)?;
            writeln!(f, "{line}")?;
        }
        f.sync_all().ok();
        self.last_flush = Instant::now();
        Ok(())
    }
}

pub(crate) fn new_chain_event(
    kind: SessionChainEventKind,
    turn_id: Option<u32>,
    tool_id: Option<String>,
    payload: serde_json::Value,
) -> SessionChainEvent {
    SessionChainEvent {
        id: Uuid::new_v4(),
        parent_id: None,
        turn_id,
        tool_id,
        kind,
        payload,
        ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }
}

pub(crate) fn replay_chain_tail(path: &Path) -> anyhow::Result<Vec<SessionChainEvent>> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let f = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut out = Vec::new();
    for line in BufReader::new(f).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(evt) = serde_json::from_str::<SessionChainEvent>(&line) {
            out.push(evt);
        }
    }
    Ok(out)
}

/// Detect tool_call_start without matching end and emit synthetic recovery events.
#[must_use]
pub(crate) fn synthesize_recovery_events(events: &[SessionChainEvent]) -> Vec<SessionChainEvent> {
    let mut open_tools: HashMap<String, &SessionChainEvent> = HashMap::new();
    let mut closed: HashSet<String> = HashSet::new();

    for evt in events {
        match evt.kind {
            SessionChainEventKind::ToolCallStart => {
                if let Some(tid) = evt.tool_id.as_deref() {
                    open_tools.insert(tid.to_string(), evt);
                }
            }
            SessionChainEventKind::ToolCallEnd | SessionChainEventKind::ToolResult => {
                if let Some(tid) = evt.tool_id.as_deref() {
                    closed.insert(tid.to_string());
                }
            }
            _ => {}
        }
    }

    let mut synth = Vec::new();
    for (tool_id, start) in open_tools {
        if closed.contains(&tool_id) {
            continue;
        }
        synth.push(SessionChainEvent {
            id: Uuid::new_v4(),
            parent_id: Some(start.id),
            turn_id: start.turn_id,
            tool_id: Some(tool_id),
            kind: SessionChainEventKind::SyntheticToolResult,
            payload: serde_json::json!({
                "reason": "recovery_unclosed_tool",
                "synthetic": true,
            }),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        });
    }
    synth
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedups_events_by_id() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        let mut chain = SessionEventChain::open(id, dir.path()).unwrap();
        let evt = new_chain_event(
            SessionChainEventKind::UserMessage,
            None,
            None,
            serde_json::json!({"text":"hi"}),
        );
        let evt_id = evt.id;
        chain.append(evt.clone()).unwrap();
        chain.append(evt).unwrap();
        chain.flush().unwrap();
        let tail = replay_chain_tail(&chain_path_for_session(id, dir.path())).unwrap();
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].id, evt_id);
    }

    #[test]
    fn synthesizes_unclosed_tool_results() {
        let start = new_chain_event(
            SessionChainEventKind::ToolCallStart,
            Some(1),
            Some("call_1".into()),
            serde_json::json!({"name":"Bash"}),
        );
        let synth = synthesize_recovery_events(&[start]);
        assert_eq!(synth.len(), 1);
        assert_eq!(synth[0].kind, SessionChainEventKind::SyntheticToolResult);
    }
}
