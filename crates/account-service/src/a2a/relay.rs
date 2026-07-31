//! In-memory streaming relay — bundle bytes pass through, never persisted to OSS/DB.
//!
//! Design (P1 correctness):
//! - Chunks are buffered so a late-connecting receiver can replay from the start.
//! - Empty chunk (`Bytes::new()`) is the EOF sentinel.
//! - Session stays open until `close_session`; callers must not mark handoff
//!   `completed` until the receiver has drained EOF.

use crate::a2a::models::MAX_RELAY_BUFFER_BYTES;
use anyhow::{anyhow, Result};
use bytes::Bytes;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use tracing::debug;

#[derive(Debug)]
struct RelayChannel {
    chunks: tokio::sync::Mutex<VecDeque<Bytes>>,
    total_bytes: tokio::sync::Mutex<usize>,
    eof: tokio::sync::Mutex<bool>,
    notify: Notify,
    stream_token: tokio::sync::Mutex<Option<String>>,
}

impl RelayChannel {
    fn new() -> Self {
        Self {
            chunks: tokio::sync::Mutex::new(VecDeque::new()),
            total_bytes: tokio::sync::Mutex::new(0),
            eof: tokio::sync::Mutex::new(false),
            notify: Notify::new(),
            stream_token: tokio::sync::Mutex::new(None),
        }
    }

    async fn publish_data(&self, chunk: Bytes) -> Result<usize> {
        if chunk.is_empty() {
            return Err(anyhow!("empty chunk reserved for EOF; use publish_eof"));
        }
        {
            let eof = self.eof.lock().await;
            if *eof {
                return Err(anyhow!("relay already closed with EOF"));
            }
        }
        {
            let mut total = self.total_bytes.lock().await;
            if *total + chunk.len() > MAX_RELAY_BUFFER_BYTES {
                return Err(anyhow!("relay buffer exceeded"));
            }
            *total += chunk.len();
        }
        {
            let mut q = self.chunks.lock().await;
            q.push_back(chunk);
        }
        self.notify.notify_waiters();
        Ok(*self.total_bytes.lock().await)
    }

    async fn publish_eof_inner(&self) -> Result<()> {
        {
            let eof = self.eof.lock().await;
            if *eof {
                return Ok(());
            }
        }
        {
            let mut q = self.chunks.lock().await;
            q.push_back(Bytes::new());
        }
        *self.eof.lock().await = true;
        self.notify.notify_waiters();
        Ok(())
    }

    async fn is_eof(&self) -> bool {
        *self.eof.lock().await
    }

    async fn total_bytes(&self) -> usize {
        *self.total_bytes.lock().await
    }
}

/// Cursor into a relay session for a single receiver.
pub struct RelaySubscription {
    handoff_id: String,
    relay: StreamRelay,
    index: usize,
}

impl RelaySubscription {
    /// Wait for the next chunk. Returns `Ok(None)` on EOF (after empty sentinel consumed).
    pub async fn next(&mut self) -> Result<Option<Bytes>> {
        loop {
            let channel = {
                let map = self.relay.inner.read().await;
                map.get(&self.handoff_id)
                    .cloned()
                    .ok_or_else(|| anyhow!("relay session not found"))?
            };
            // Register waiter BEFORE checking to avoid lost EOF wakeups.
            let notified = channel.notify.notified();
            {
                let q = channel.chunks.lock().await;
                if self.index < q.len() {
                    let chunk = q[self.index].clone();
                    self.index += 1;
                    drop(q);
                    drop(notified);
                    if chunk.is_empty() {
                        return Ok(None);
                    }
                    return Ok(Some(chunk));
                }
            }
            if channel.is_eof().await {
                drop(notified);
                return Ok(None);
            }
            notified.await;
        }
    }
}

#[derive(Clone)]
pub struct StreamRelay {
    inner: Arc<RwLock<HashMap<String, Arc<RelayChannel>>>>,
}

impl Default for StreamRelay {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl StreamRelay {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn open_session(&self, handoff_id: &str) {
        let mut map = self.inner.write().await;
        map.entry(handoff_id.to_string())
            .or_insert_with(|| Arc::new(RelayChannel::new()));
    }

    pub async fn publish_chunk(&self, handoff_id: &str, chunk: Bytes) -> Result<usize> {
        let channel = {
            let map = self.inner.read().await;
            map.get(handoff_id)
                .cloned()
                .ok_or_else(|| anyhow!("relay session not found"))?
        };
        channel.publish_data(chunk).await
    }

    /// Publish EOF sentinel (empty chunk). Idempotent if already EOF.
    pub async fn publish_eof(&self, handoff_id: &str) -> Result<()> {
        let channel = {
            let map = self.inner.read().await;
            map.get(handoff_id)
                .cloned()
                .ok_or_else(|| anyhow!("relay session not found"))?
        };
        channel.publish_eof_inner().await
    }

    pub async fn subscribe(&self, handoff_id: &str) -> Result<RelaySubscription> {
        let map = self.inner.read().await;
        if !map.contains_key(handoff_id) {
            return Err(anyhow!("relay session not found"));
        }
        Ok(RelaySubscription {
            handoff_id: handoff_id.to_string(),
            relay: self.clone(),
            index: 0,
        })
    }

    pub async fn close_session(&self, handoff_id: &str) {
        let mut map = self.inner.write().await;
        map.remove(handoff_id);
        debug!(handoff_id, "relay session closed");
    }

    pub async fn session_exists(&self, handoff_id: &str) -> bool {
        self.inner.read().await.contains_key(handoff_id)
    }

    pub async fn store_stream_token(&self, handoff_id: &str, token: String) {
        let mut map = self.inner.write().await;
        let channel = map
            .entry(handoff_id.to_string())
            .or_insert_with(|| Arc::new(RelayChannel::new()));
        *channel.stream_token.lock().await = Some(token);
    }

    pub async fn get_stream_token(&self, handoff_id: &str) -> Option<String> {
        let channel = {
            let map = self.inner.read().await;
            map.get(handoff_id)?.clone()
        };
        let guard = channel.stream_token.lock().await;
        guard.clone()
    }

    pub async fn clear_stream_token(&self, handoff_id: &str) {
        let Some(channel) = ({
            let map = self.inner.read().await;
            map.get(handoff_id).cloned()
        }) else {
            return;
        };
        *channel.stream_token.lock().await = None;
    }

    pub async fn total_bytes(&self, handoff_id: &str) -> Option<usize> {
        let channel = {
            let map = self.inner.read().await;
            map.get(handoff_id)?.clone()
        };
        Some(channel.total_bytes().await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::a2a::models::MAX_RELAY_BUFFER_BYTES;

    #[tokio::test]
    async fn relay_pipes_chunks() {
        let relay = StreamRelay::new();
        relay.open_session("ho_test").await;
        let mut rx = relay.subscribe("ho_test").await.unwrap();
        relay
            .publish_chunk("ho_test", Bytes::from_static(b"hello"))
            .await
            .unwrap();
        let chunk = rx.next().await.unwrap().unwrap();
        assert_eq!(&chunk[..], b"hello");
        relay.publish_eof("ho_test").await.unwrap();
        assert!(rx.next().await.unwrap().is_none());
        relay.close_session("ho_test").await;
    }

    #[tokio::test]
    async fn late_subscriber_replays_buffered_chunks() {
        let relay = StreamRelay::new();
        relay.open_session("ho_late").await;
        relay
            .publish_chunk("ho_late", Bytes::from_static(b"a"))
            .await
            .unwrap();
        relay
            .publish_chunk("ho_late", Bytes::from_static(b"b"))
            .await
            .unwrap();
        relay.publish_eof("ho_late").await.unwrap();

        // Receiver connects after EOF — must still get full payload.
        let mut rx = relay.subscribe("ho_late").await.unwrap();
        assert_eq!(&rx.next().await.unwrap().unwrap()[..], b"a");
        assert_eq!(&rx.next().await.unwrap().unwrap()[..], b"b");
        assert!(rx.next().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn multi_chunk_then_eof() {
        let relay = StreamRelay::new();
        relay.open_session("ho_multi").await;
        let mut rx = relay.subscribe("ho_multi").await.unwrap();
        for i in 0..8u8 {
            relay
                .publish_chunk("ho_multi", Bytes::from(vec![i]))
                .await
                .unwrap();
        }
        relay.publish_eof("ho_multi").await.unwrap();
        let mut got = Vec::new();
        while let Some(c) = rx.next().await.unwrap() {
            got.extend_from_slice(&c);
        }
        assert_eq!(got, (0..8u8).collect::<Vec<_>>());
    }

    #[tokio::test]
    async fn publish_chunk_rejects_empty() {
        let relay = StreamRelay::new();
        relay.open_session("ho_empty").await;
        let err = relay
            .publish_chunk("ho_empty", Bytes::new())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("EOF"));
    }

    #[tokio::test]
    async fn buffer_cap_rejects_oversized() {
        let relay = StreamRelay::new();
        relay.open_session("ho_cap").await;
        // Publish just under cap, then one more byte → fail.
        let big = Bytes::from(vec![0u8; MAX_RELAY_BUFFER_BYTES]);
        relay.publish_chunk("ho_cap", big).await.unwrap();
        let err = relay
            .publish_chunk("ho_cap", Bytes::from_static(b"x"))
            .await
            .unwrap_err();
        assert!(err.to_string().contains("exceeded"));
    }

    #[tokio::test]
    async fn stream_token_survives_until_cleared() {
        let relay = StreamRelay::new();
        relay.store_stream_token("ho_tok", "secret".into()).await;
        assert_eq!(
            relay.get_stream_token("ho_tok").await.as_deref(),
            Some("secret")
        );
        relay.clear_stream_token("ho_tok").await;
        assert!(relay.get_stream_token("ho_tok").await.is_none());
    }
}
