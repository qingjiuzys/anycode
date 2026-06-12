//! In-memory LRU cache for assembled session transcripts.

use crate::schema::SessionTranscriptResponse;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_ENTRIES: usize = 128;
const RUNNING_TTL: Duration = Duration::from_secs(2);
const COMPLETED_TTL: Duration = Duration::from_secs(600);

#[derive(Clone)]
struct CacheEntry {
    transcript: SessionTranscriptResponse,
    cached_at: Instant,
    event_count: usize,
    last_event_id: String,
    is_running: bool,
}

struct CacheInner {
    entries: HashMap<String, CacheEntry>,
}

impl CacheInner {
    fn new() -> Self {
        Self {
            entries: HashMap::with_capacity(MAX_ENTRIES),
        }
    }

    fn get(
        &self,
        session_id: &str,
        event_count: usize,
        last_event_id: &str,
        is_running: bool,
    ) -> Option<SessionTranscriptResponse> {
        let entry = self.entries.get(session_id)?;
        let ttl = if is_running {
            RUNNING_TTL
        } else {
            COMPLETED_TTL
        };
        if entry.cached_at.elapsed() > ttl {
            return None;
        }
        if entry.event_count != event_count || entry.last_event_id != last_event_id {
            return None;
        }
        if entry.is_running != is_running {
            return None;
        }
        Some(entry.transcript.clone())
    }

    fn put(
        &mut self,
        session_id: &str,
        transcript: SessionTranscriptResponse,
        event_count: usize,
        last_event_id: String,
        is_running: bool,
    ) {
        if self.entries.len() >= MAX_ENTRIES && !self.entries.contains_key(session_id) {
            if let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, v)| v.cached_at)
                .map(|(k, _)| k.clone())
            {
                self.entries.remove(&oldest_key);
            }
        }
        self.entries.insert(
            session_id.to_string(),
            CacheEntry {
                transcript,
                cached_at: Instant::now(),
                event_count,
                last_event_id,
                is_running,
            },
        );
    }

    fn invalidate(&mut self, session_id: &str) {
        self.entries.remove(session_id);
    }
}

fn cache() -> &'static Mutex<CacheInner> {
    static CACHE: OnceLock<Mutex<CacheInner>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(CacheInner::new()))
}

pub fn get_cached(
    session_id: &str,
    event_count: usize,
    last_event_id: &str,
    is_running: bool,
) -> Option<SessionTranscriptResponse> {
    cache()
        .lock()
        .ok()?
        .get(session_id, event_count, last_event_id, is_running)
}

pub fn put_cached(
    session_id: &str,
    transcript: SessionTranscriptResponse,
    event_count: usize,
    last_event_id: String,
    is_running: bool,
) {
    if let Ok(mut inner) = cache().lock() {
        inner.put(
            session_id,
            transcript,
            event_count,
            last_event_id,
            is_running,
        );
    }
}

pub fn invalidate_session(session_id: &str) {
    if let Ok(mut inner) = cache().lock() {
        inner.invalidate(session_id);
    }
}

pub fn event_fingerprint(events: &[crate::schema::ProjectEvent]) -> (usize, String) {
    let count = events.len();
    let last_id = events.last().map(|e| e.id.clone()).unwrap_or_default();
    (count, last_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{SessionTranscriptResponse, TranscriptBlock};

    fn sample_transcript(session_id: &str) -> SessionTranscriptResponse {
        SessionTranscriptResponse {
            schema_version: 1,
            session_id: session_id.into(),
            blocks: vec![TranscriptBlock {
                id: "b1".into(),
                block_type: "user_message".into(),
                at: "2026-01-01T00:00:00Z".into(),
                title: "You".into(),
                body: "hi".into(),
                meta: serde_json::json!({}),
                collapsible: false,
                default_collapsed: false,
                event_id: None,
            }],
            lifecycle: vec![],
        }
    }

    #[test]
    fn cache_hit_and_invalidate() {
        invalidate_session("sess-a");
        let t = sample_transcript("sess-a");
        put_cached("sess-a", t.clone(), 2, "evt_last".into(), false);
        assert!(get_cached("sess-a", 2, "evt_last", false).is_some());
        assert!(get_cached("sess-a", 3, "evt_last", false).is_none());
        invalidate_session("sess-a");
        assert!(get_cached("sess-a", 2, "evt_last", false).is_none());
    }
}
