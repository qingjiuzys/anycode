//! 归根通道：虚态缓冲 → 强化晋升 → 热层（Sled）→ 可选向量层。

use crate::buffer_wal::BufferWal;
use crate::retrieval::{KeywordRetrieval, MemoryRetrieval};
use crate::{MemoryError, SledMemoryStore};
use anycode_core::prelude::*;
use anycode_core::{
    EmbeddingProvider, MemoryPipeline, MemoryPipelineSettings, PreSemanticFragment,
    VectorMemoryBackend,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// 无操作的向量后端（默认）：不持久化向量、检索恒为空。
pub struct NoopVectorBackend;

#[async_trait]
impl VectorMemoryBackend for NoopVectorBackend {
    async fn upsert(
        &self,
        _id: &str,
        _embedding: &[f32],
        _mem_type: MemoryType,
    ) -> Result<(), CoreError> {
        Ok(())
    }

    async fn search(
        &self,
        _query_embedding: &[f32],
        _mem_type: MemoryType,
        _limit: usize,
    ) -> Result<Vec<String>, CoreError> {
        Ok(vec![])
    }

    async fn remove(&self, _id: &str, _mem_type: MemoryType) -> Result<(), CoreError> {
        Ok(())
    }
}

/// 无嵌入：向量检索路径不启用。
pub struct NoopEmbeddingProvider;

#[async_trait]
impl EmbeddingProvider for NoopEmbeddingProvider {
    async fn embed_one(&self, _text: &str) -> Result<Vec<f32>, CoreError> {
        Err(CoreError::Other(anyhow::anyhow!("embedding disabled")))
    }
}

fn core_from_mem(e: MemoryError) -> CoreError {
    CoreError::Other(anyhow::anyhow!(e))
}

fn fragment_matches_query(frag: &PreSemanticFragment, query: &str) -> bool {
    let q = query.trim();
    if q.is_empty() {
        return true;
    }
    frag.raw_text.contains(q)
}

fn synthetic_memory_from_fragment(frag: &PreSemanticFragment) -> Memory {
    let line0 = frag.raw_text.lines().next().unwrap_or("").trim();
    let title = if line0.chars().count() > 120 {
        line0.chars().take(120).collect::<String>() + "…"
    } else if line0.is_empty() {
        "(buffer fragment)".to_string()
    } else {
        line0.to_string()
    };
    Memory {
        id: format!("buf:{}", frag.id),
        mem_type: frag.mem_type,
        title,
        content: frag.raw_text.clone(),
        tags: vec!["pre-semantic".to_string()],
        scope: MemoryScope::Project,
        created_at: frag.created_at,
        updated_at: frag.last_touched_at,
    }
}

/// 归根通道管线 + 可作为 [`MemoryStore`] 使用（`recall` 即 [`MemoryPipeline::materialize_for_prompt`]）。
pub struct RootReturnMemoryPipeline {
    settings: MemoryPipelineSettings,
    buffer: Arc<RwLock<HashMap<String, PreSemanticFragment>>>,
    wal: Option<Arc<BufferWal>>,
    hot: Arc<SledMemoryStore>,
    legacy_file: Option<Arc<crate::FileMemoryStore>>,
    vector: Arc<dyn VectorMemoryBackend>,
    embedding: Option<Arc<dyn EmbeddingProvider>>,
}

impl RootReturnMemoryPipeline {
    /// 打开管线：可选从 WAL 重放虚态缓冲。
    pub fn open(
        settings: MemoryPipelineSettings,
        hot_sled_path: impl Into<std::path::PathBuf>,
        buffer_wal_path: Option<std::path::PathBuf>,
        legacy_file: Option<Arc<crate::FileMemoryStore>>,
        vector: Arc<dyn VectorMemoryBackend>,
        embedding: Option<Arc<dyn EmbeddingProvider>>,
    ) -> Result<Self, MemoryError> {
        let hot_path = hot_sled_path.into();
        let (buffer, wal) = if settings.buffer_wal_enabled {
            if let Some(wal_path) = buffer_wal_path {
                let initial = BufferWal::replay(&wal_path).unwrap_or_default();
                let n = settings.buffer_wal_fsync_every_n.max(1);
                let w = Arc::new(BufferWal::open(&wal_path, n)?);
                (Arc::new(RwLock::new(initial)), Some(w))
            } else {
                (Arc::new(RwLock::new(HashMap::new())), None)
            }
        } else {
            (Arc::new(RwLock::new(HashMap::new())), None)
        };

        Ok(Self {
            settings,
            buffer,
            wal,
            hot: Arc::new(SledMemoryStore::new(hot_path)?),
            legacy_file,
            vector,
            embedding,
        })
    }

    async fn wal_put(&self, frag: &PreSemanticFragment) {
        if let Some(ref w) = self.wal {
            if let Err(e) = w.append_put(frag) {
                tracing::warn!(target: "anycode_memory", "wal append_put: {}", e);
            }
        }
    }

    async fn wal_del(&self, id: &str) {
        if let Some(ref w) = self.wal {
            if let Err(e) = w.append_delete(id) {
                tracing::warn!(target: "anycode_memory", "wal append_delete: {}", e);
            }
        }
    }

    async fn evict_buffer_if_full(&self) {
        let max = self.settings.max_buffer_fragments.max(1);
        loop {
            let victim = {
                let buf = self.buffer.read().await;
                if buf.len() < max {
                    return;
                }
                buf.iter()
                    .min_by_key(|(_, f)| f.last_touched_at)
                    .map(|(k, _)| k.clone())
            };
            let Some(k) = victim else {
                return;
            };
            {
                let mut buf = self.buffer.write().await;
                buf.remove(&k);
            }
            self.wal_del(&k).await;
        }
    }

    async fn promote_fragment(&self, frag: PreSemanticFragment) -> Result<(), CoreError> {
        let line0 = frag.raw_text.lines().next().unwrap_or("").trim();
        let title = if line0.chars().count() > 200 {
            line0.chars().take(200).collect::<String>() + "…"
        } else if line0.is_empty() {
            "(promoted fragment)".to_string()
        } else {
            line0.to_string()
        };
        let memory = Memory {
            id: frag.id.clone(),
            mem_type: frag.mem_type,
            title,
            content: frag.raw_text.clone(),
            tags: vec![],
            scope: MemoryScope::Project,
            created_at: frag.created_at,
            updated_at: chrono::Utc::now(),
        };
        self.hot.save(memory.clone()).await?;
        if let Some(ref emb) = self.embedding {
            if let Ok(vec) = emb.embed_one(&memory.content).await {
                let _ = self.vector.upsert(&memory.id, &vec, memory.mem_type).await;
            }
        }
        Ok(())
    }

    async fn touch_internal(&self, id: &str) -> Result<(), CoreError> {
        let mut buf = self.buffer.write().await;
        let Some(mut frag) = buf.get(id).cloned() else {
            return Ok(());
        };
        frag.touch_count = frag.touch_count.saturating_add(1);
        frag.last_touched_at = chrono::Utc::now();
        let threshold = self.settings.promote_touch_threshold.max(1);
        if frag.touch_count >= threshold {
            buf.remove(id);
            drop(buf);
            self.wal_del(id).await;
            self.promote_fragment(frag).await?;
        } else {
            buf.insert(id.to_string(), frag.clone());
            drop(buf);
            self.wal_put(&frag).await;
        }
        Ok(())
    }

    fn tick_decay_locked(
        settings: &MemoryPipelineSettings,
        buf: &mut HashMap<String, PreSemanticFragment>,
    ) {
        let now = chrono::Utc::now();
        let ttl = chrono::Duration::seconds(settings.buffer_ttl_secs as i64);
        buf.retain(|_, frag| now.signed_duration_since(frag.last_touched_at) <= ttl);
    }

    /// 虚态缓冲 WAL 刷盘（显式调用或 [`Drop`]）。
    pub fn flush_wal(&self) -> std::io::Result<()> {
        if let Some(ref w) = self.wal {
            w.sync_all()?;
        }
        Ok(())
    }
}

impl Drop for RootReturnMemoryPipeline {
    fn drop(&mut self) {
        if let Err(e) = self.flush_wal() {
            tracing::warn!(target: "anycode_memory", "wal shutdown sync: {}", e);
        }
    }
}

#[async_trait]
impl MemoryPipeline for RootReturnMemoryPipeline {
    async fn ingest_fragment(
        &self,
        session_id: &str,
        text: &str,
        mem_type: MemoryType,
    ) -> Result<String, CoreError> {
        self.evict_buffer_if_full().await;
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now();
        let frag = PreSemanticFragment {
            id: id.clone(),
            session_id: session_id.to_string(),
            mem_type,
            raw_text: text.to_string(),
            created_at: now,
            last_touched_at: now,
            touch_count: 1,
        };
        {
            let mut buf = self.buffer.write().await;
            buf.insert(id.clone(), frag.clone());
        }
        self.wal_put(&frag).await;
        Ok(id)
    }

    async fn touch(&self, fragment_id: &str) -> Result<(), CoreError> {
        self.touch_internal(fragment_id).await
    }

    async fn tick_decay(&self) -> Result<(), CoreError> {
        let removed: Vec<String> = {
            let mut buf = self.buffer.write().await;
            let before: Vec<String> = buf.keys().cloned().collect();
            Self::tick_decay_locked(&self.settings, &mut buf);
            let after: std::collections::HashSet<String> = buf.keys().cloned().collect();
            before.into_iter().filter(|k| !after.contains(k)).collect()
        };
        for id in removed {
            self.wal_del(&id).await;
        }
        Ok(())
    }

    async fn materialize_for_prompt(
        &self,
        query: &str,
        mem_type: MemoryType,
    ) -> Result<Vec<Memory>, CoreError> {
        self.tick_decay().await?;

        let matching_ids: Vec<String> = {
            let buf = self.buffer.read().await;
            buf.iter()
                .filter(|(_, f)| f.mem_type == mem_type && fragment_matches_query(f, query))
                .map(|(id, _)| id.clone())
                .collect()
        };

        if self.settings.reinforce_on_recall_match {
            for id in &matching_ids {
                let _ = self.touch_internal(id).await;
            }
        }

        let from_buffer: Vec<Memory> = {
            let buf = self.buffer.read().await;
            buf.iter()
                .filter(|(_, f)| f.mem_type == mem_type && fragment_matches_query(f, query))
                .map(|(_, f)| synthetic_memory_from_fragment(f))
                .collect()
        };

        let from_hot = self.hot.recall(query, mem_type).await?;

        let mut from_vector: Vec<Memory> = Vec::new();
        if let Some(ref emb) = self.embedding {
            if let Ok(qvec) = emb.embed_one(query).await {
                let ids = self
                    .vector
                    .search(&qvec, mem_type, 32)
                    .await
                    .unwrap_or_default();
                for id in ids {
                    if let Ok(Some(m)) = self.hot.get_by_id(&id, &mem_type).map_err(core_from_mem) {
                        from_vector.push(m);
                    }
                }
            }
        }

        let mut from_legacy: Vec<Memory> = Vec::new();
        if self.settings.merge_legacy_file_recall {
            if let Some(ref file) = self.legacy_file {
                from_legacy = file.recall(query, mem_type).await?;
            }
        }

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut merged: Vec<Memory> = Vec::new();

        for m in from_vector.into_iter().chain(from_hot) {
            if seen.insert(m.id.clone()) {
                merged.push(m);
            }
        }
        for m in from_buffer {
            let key = m.id.strip_prefix("buf:").unwrap_or(&m.id).to_string();
            if seen.insert(key) {
                merged.push(m);
            }
        }
        for m in from_legacy {
            if seen.insert(m.id.clone()) {
                merged.push(m);
            }
        }

        Ok(KeywordRetrieval.rank(query, merged))
    }

    async fn promote_fragment_to_hot(&self, fragment_id: &str) -> Result<(), CoreError> {
        let frag = {
            let mut buf = self.buffer.write().await;
            buf.remove(fragment_id)
        };
        let Some(frag) = frag else {
            return Err(CoreError::Other(anyhow::anyhow!(
                "fragment not in buffer: {}",
                fragment_id
            )));
        };
        self.wal_del(fragment_id).await;
        self.promote_fragment(frag).await
    }

    async fn promote_memory_to_vector(
        &self,
        memory_id: &str,
        mem_type: MemoryType,
    ) -> Result<(), CoreError> {
        let Some(ref emb) = self.embedding else {
            return Err(CoreError::Other(anyhow::anyhow!(
                "embedding not configured"
            )));
        };
        let Some(m) = self
            .hot
            .get_by_id(memory_id, &mem_type)
            .map_err(core_from_mem)?
        else {
            return Err(CoreError::Other(anyhow::anyhow!("memory not in hot store")));
        };
        let vec = emb.embed_one(&m.content).await?;
        self.vector.upsert(&m.id, &vec, m.mem_type).await
    }

    fn sync_durability(&self) -> Result<(), CoreError> {
        self.flush_wal()
            .map_err(|e| CoreError::Other(anyhow::anyhow!("wal sync: {}", e)))
    }
}

#[async_trait]
impl MemoryStore for RootReturnMemoryPipeline {
    async fn save(&self, memory: Memory) -> Result<(), CoreError> {
        {
            let mut buf = self.buffer.write().await;
            buf.retain(|_, f| f.id != memory.id);
        }
        self.wal_del(&memory.id).await;
        self.hot.save(memory.clone()).await?;
        if let Some(ref emb) = self.embedding {
            if let Ok(vec) = emb.embed_one(&memory.content).await {
                let _ = self.vector.upsert(&memory.id, &vec, memory.mem_type).await;
            }
        }
        Ok(())
    }

    async fn recall(&self, query: &str, mem_type: MemoryType) -> Result<Vec<Memory>, CoreError> {
        self.materialize_for_prompt(query, mem_type).await
    }

    async fn update(&self, id: &str, memory: Memory) -> Result<(), CoreError> {
        {
            let mut buf = self.buffer.write().await;
            buf.remove(id);
        }
        self.wal_del(id).await;
        self.hot.update(id, memory.clone()).await?;
        if let Some(ref emb) = self.embedding {
            if let Ok(vec) = emb.embed_one(&memory.content).await {
                let _ = self.vector.upsert(&memory.id, &vec, memory.mem_type).await;
            }
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), CoreError> {
        {
            let mut buf = self.buffer.write().await;
            buf.remove(id);
        }
        self.wal_del(id).await;
        for t in MemoryType::ALL {
            let _ = self.vector.remove(id, t).await;
        }
        self.hot.delete(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_sled(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("anycode-pipeline-test-{}-{}", name, Uuid::new_v4()))
    }

    fn test_settings() -> MemoryPipelineSettings {
        let mut s = MemoryPipelineSettings::default();
        s.buffer_wal_enabled = false;
        s
    }

    #[tokio::test]
    async fn promote_on_recall_when_threshold_met() {
        let sled = temp_sled("promote");
        let mut settings = test_settings();
        settings.promote_touch_threshold = 2;
        settings.reinforce_on_recall_match = true;
        settings.merge_legacy_file_recall = false;

        let pipe = RootReturnMemoryPipeline::open(
            settings,
            sled,
            None,
            None,
            Arc::new(NoopVectorBackend),
            None,
        )
        .expect("pipeline new");

        let _id = pipe
            .ingest_fragment("sess", "unique_alpha_token hello", MemoryType::Project)
            .await
            .expect("ingest");

        let out = pipe
            .materialize_for_prompt("unique_alpha_token", MemoryType::Project)
            .await
            .expect("materialize");
        assert!(
            out.iter().any(|m| m.content.contains("unique_alpha_token")),
            "expected promoted or buffer memory"
        );

        let out2 = pipe
            .materialize_for_prompt("unique_alpha_token", MemoryType::Project)
            .await
            .expect("materialize2");
        assert!(
            out2.iter().any(|m| !m.id.starts_with("buf:")),
            "expected hot-layer memory after promotion"
        );
    }

    #[tokio::test]
    async fn tick_decay_drops_stale_buffer() {
        let sled = temp_sled("decay");
        let mut settings = test_settings();
        settings.buffer_ttl_secs = 0;
        settings.merge_legacy_file_recall = false;

        let pipe = Arc::new(
            RootReturnMemoryPipeline::open(
                settings,
                sled,
                None,
                None,
                Arc::new(NoopVectorBackend),
                None,
            )
            .expect("pipeline new"),
        );

        pipe.ingest_fragment("s", "stale_blob", MemoryType::Project)
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        pipe.tick_decay().await.unwrap();
        let out = pipe
            .materialize_for_prompt("stale_blob", MemoryType::Project)
            .await
            .unwrap();
        assert!(
            out.is_empty(),
            "zero TTL should evict buffer before hot promotion"
        );
    }

    #[tokio::test]
    async fn promote_fragment_to_hot_explicit() {
        let sled = temp_sled("pexplicit");
        let mut settings = test_settings();
        settings.merge_legacy_file_recall = false;
        let pipe = RootReturnMemoryPipeline::open(
            settings,
            sled,
            None,
            None,
            Arc::new(NoopVectorBackend),
            None,
        )
        .unwrap();
        let fid = pipe
            .ingest_fragment("s", "explicit_body_xyz", MemoryType::Project)
            .await
            .unwrap();
        MemoryPipeline::promote_fragment_to_hot(&pipe, &fid)
            .await
            .unwrap();
        let m = pipe
            .hot
            .get_by_id(&fid, &MemoryType::Project)
            .unwrap()
            .unwrap();
        assert!(m.content.contains("explicit_body_xyz"));
    }

    /// `buffer_wal_fsync_every_n` 很大时单条写入不会触发 periodic fsync；drop 刷盘后重启应重放缓冲。
    #[tokio::test]
    async fn wal_drop_flushes_so_reopen_replays_buffer() {
        let base = temp_sled("wal-drop");
        let wal_path = std::path::PathBuf::from(format!("{}.buffer.wal", base.display()));
        let _ = std::fs::remove_file(&wal_path);
        let mut settings = test_settings();
        settings.buffer_wal_enabled = true;
        settings.buffer_wal_fsync_every_n = 10_000;
        settings.merge_legacy_file_recall = false;

        let token = "wal_reopen_token_qwerty";
        {
            let pipe = RootReturnMemoryPipeline::open(
                settings.clone(),
                base.clone(),
                Some(wal_path.clone()),
                None,
                Arc::new(NoopVectorBackend),
                None,
            )
            .expect("open pipeline");
            pipe.ingest_fragment("sess", token, MemoryType::Project)
                .await
                .expect("ingest");
        }

        let pipe2 = RootReturnMemoryPipeline::open(
            settings,
            base,
            Some(wal_path),
            None,
            Arc::new(NoopVectorBackend),
            None,
        )
        .expect("reopen");
        let out = pipe2
            .materialize_for_prompt(token, MemoryType::Project)
            .await
            .expect("materialize");
        assert!(
            out.iter().any(|m| m.content.contains(token)),
            "buffer should replay from WAL after drop"
        );
    }
}
