//! 热层侧车 Sled：存 embedding，检索用余弦相似度线性扫描（适合万级以内；大规模可换 ANN）。

use anycode_core::prelude::*;
use anycode_core::VectorMemoryBackend;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sled::Db;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
struct StoredEmb {
    id: String,
    mem_type: MemoryType,
    vec: Vec<f32>,
}

fn mem_type_key_byte(t: MemoryType) -> u8 {
    t.discriminant()
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return f32::NAN;
    }
    let mut dot = 0f32;
    let mut na = 0f32;
    let mut nb = 0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let d = (na.sqrt() * nb.sqrt()).max(1e-12);
    dot / d
}

pub struct SledVectorBackend {
    db: Arc<Db>,
}

impl SledVectorBackend {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, sled::Error> {
        let db = sled::open(path.into())?;
        Ok(Self { db: Arc::new(db) })
    }

    fn tree_key(id: &str, mem_type: MemoryType) -> Vec<u8> {
        let mut k = vec![mem_type_key_byte(mem_type)];
        k.extend_from_slice(id.as_bytes());
        k
    }
}

#[async_trait]
impl VectorMemoryBackend for SledVectorBackend {
    async fn upsert(
        &self,
        id: &str,
        embedding: &[f32],
        mem_type: MemoryType,
    ) -> Result<(), CoreError> {
        let db = self.db.clone();
        let id = id.to_string();
        let emb = embedding.to_vec();
        tokio::task::spawn_blocking(move || {
            let rec = StoredEmb {
                id: id.clone(),
                mem_type,
                vec: emb,
            };
            let v = serde_json::to_vec(&rec).map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?;
            db.insert(Self::tree_key(&id, mem_type), v)
                .map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?;
            db.flush()
                .map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?;
            Ok::<(), CoreError>(())
        })
        .await
        .map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?
    }

    async fn search(
        &self,
        query_embedding: &[f32],
        mem_type: MemoryType,
        limit: usize,
    ) -> Result<Vec<String>, CoreError> {
        let db = self.db.clone();
        let q = query_embedding.to_vec();
        let want = mem_type_key_byte(mem_type);
        tokio::task::spawn_blocking(move || {
            let mut scored: Vec<(f32, String)> = Vec::new();
            for item in db.iter() {
                let (k, v) = item.map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?;
                if k.is_empty() || k[0] != want {
                    continue;
                }
                let rec: StoredEmb =
                    serde_json::from_slice(&v).map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?;
                let s = cosine(&q, &rec.vec);
                if s.is_finite() {
                    scored.push((s, rec.id));
                }
            }
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            Ok(scored
                .into_iter()
                .take(limit.max(1))
                .map(|(_, id)| id)
                .collect())
        })
        .await
        .map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?
    }

    async fn remove(&self, id: &str, mem_type: MemoryType) -> Result<(), CoreError> {
        let db = self.db.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            db.remove(Self::tree_key(&id, mem_type))
                .map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?;
            let _ = db.flush();
            Ok::<(), CoreError>(())
        })
        .await
        .map_err(|e| CoreError::Other(anyhow::anyhow!(e)))?
    }
}
