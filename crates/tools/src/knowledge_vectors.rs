//! Optional local embedding vectors for project knowledge (FastEmbed + Sled sidecar).
//!
//! Enable with `anycode-tools/knowledge-embeddings` (pulls `anycode-memory/embedding-local`).

#[cfg(feature = "knowledge-embeddings")]
use anyhow::Context;
use anyhow::Result;
#[cfg(feature = "knowledge-embeddings")]
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "knowledge-embeddings")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredChunk {
    source_file: String,
    snippet: String,
    vec: Vec<f32>,
}

#[derive(Debug, Clone)]
pub struct VectorHit {
    pub source_file: String,
    pub snippet: String,
    pub score: f32,
}

pub fn vectors_feature_enabled() -> bool {
    cfg!(feature = "knowledge-embeddings")
}

pub fn vector_store_path(project_id: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join(".anycode/knowledge")
            .join(project_id)
            .join("vec.sled")
    })
}

pub fn vector_chunk_count(project_id: &str) -> usize {
    #[cfg(feature = "knowledge-embeddings")]
    {
        let Some(path) = vector_store_path(project_id) else {
            return 0;
        };
        if !path.is_dir() {
            return 0;
        }
        match sled::open(&path) {
            Ok(db) => db.len(),
            Err(_) => 0,
        }
    }
    #[cfg(not(feature = "knowledge-embeddings"))]
    {
        let _ = project_id;
        0
    }
}

#[cfg(feature = "knowledge-embeddings")]
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

/// Merge BM25 and vector hits by source file + snippet prefix; hybrid score = 0.65*vec + 0.35*bm25 (normalized).
pub fn merge_hybrid_knowledge_hits<T>(
    bm25: Vec<T>,
    vectors: Vec<VectorHit>,
    limit: usize,
    source: impl Fn(&T) -> &str,
    snippet: impl Fn(&T) -> &str,
    score: impl Fn(&T) -> f32,
    map: impl Fn(String, String, f32) -> T,
) -> Vec<T> {
    if vectors.is_empty() {
        let mut out = bm25;
        out.truncate(limit);
        return out;
    }
    let max_bm25 = bm25
        .iter()
        .map(|h| score(h))
        .fold(0.0_f32, f32::max)
        .max(1e-6);
    let max_vec = vectors
        .iter()
        .map(|h| h.score)
        .fold(0.0_f32, f32::max)
        .max(1e-6);

    let mut merged: std::collections::HashMap<String, (String, f32)> =
        std::collections::HashMap::new();
    for h in bm25 {
        let key = format!(
            "{}::{}",
            source(&h),
            snippet(&h).chars().take(64).collect::<String>()
        );
        let norm = score(&h) / max_bm25;
        merged.insert(key, (snippet(&h).to_string(), 0.35 * norm));
    }
    for v in vectors {
        let key = format!(
            "{}::{}",
            v.source_file,
            v.snippet.chars().take(64).collect::<String>()
        );
        let norm = v.score / max_vec;
        merged
            .entry(key)
            .and_modify(|(_, s)| *s += 0.65 * norm)
            .or_insert((v.snippet.clone(), 0.65 * norm));
    }
    let mut rows: Vec<(String, String, f32)> = merged
        .into_iter()
        .map(|(k, (snip, sc))| {
            let src = k.split("::").next().unwrap_or("").to_string();
            (src, snip, sc)
        })
        .collect();
    rows.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    rows.truncate(limit);
    rows.into_iter()
        .map(|(src, snip, sc)| map(src, snip, sc))
        .collect()
}

#[cfg(not(feature = "knowledge-embeddings"))]
pub async fn rebuild_project_vectors(
    _project_id: &str,
    _chunks: &[(String, String, String)],
) -> Result<usize> {
    Ok(0)
}

#[cfg(not(feature = "knowledge-embeddings"))]
pub async fn search_project_vectors(
    _project_id: &str,
    _query: &str,
    _limit: usize,
) -> Result<Vec<VectorHit>> {
    Ok(vec![])
}

#[cfg(feature = "knowledge-embeddings")]
pub async fn rebuild_project_vectors(
    project_id: &str,
    chunks: &[(String, String, String)],
) -> Result<usize> {
    use anycode_core::EmbeddingProvider;
    use anycode_memory::FastEmbedEmbeddingProvider;

    let Some(path) = vector_store_path(project_id) else {
        return Ok(0);
    };
    if path.exists() {
        std::fs::remove_dir_all(&path).with_context(|| format!("clear {}", path.display()))?;
    }
    std::fs::create_dir_all(path.parent().unwrap_or(&path))?;
    let db = sled::open(&path).with_context(|| format!("open sled {}", path.display()))?;
    let embedder =
        FastEmbedEmbeddingProvider::try_new(None, None).context("init FastEmbed embedder")?;
    let mut count = 0usize;
    for (id, source_file, content) in chunks {
        let text = content.trim();
        if text.is_empty() {
            continue;
        }
        let vec = embedder.embed_one(text).await.context("embed chunk")?;
        let snippet: String = text.chars().take(400).collect();
        let rec = StoredChunk {
            source_file: source_file.clone(),
            snippet,
            vec,
        };
        let bytes = serde_json::to_vec(&rec).context("serialize chunk vec")?;
        db.insert(id.as_bytes(), bytes)
            .with_context(|| format!("sled insert {id}"))?;
        count += 1;
    }
    db.flush().context("sled flush")?;
    Ok(count)
}

#[cfg(feature = "knowledge-embeddings")]
pub async fn search_project_vectors(
    project_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<VectorHit>> {
    use anycode_core::EmbeddingProvider;
    use anycode_memory::FastEmbedEmbeddingProvider;

    let q = query.trim();
    if q.is_empty() || limit == 0 {
        return Ok(vec![]);
    }
    let Some(path) = vector_store_path(project_id) else {
        return Ok(vec![]);
    };
    if !path.is_dir() {
        return Ok(vec![]);
    }
    let db = sled::open(&path).with_context(|| format!("open sled {}", path.display()))?;
    let embedder =
        FastEmbedEmbeddingProvider::try_new(None, None).context("init FastEmbed embedder")?;
    let q_vec = embedder.embed_one(q).await.context("embed query")?;
    let db_c = db.clone();
    let q_vec_c = q_vec.clone();
    let scored: Vec<VectorHit> = tokio::task::spawn_blocking(move || {
        let mut hits = Vec::new();
        for item in db_c.iter().flatten() {
            let Ok(rec) = serde_json::from_slice::<StoredChunk>(&item.1) else {
                continue;
            };
            let score = cosine(&q_vec_c, &rec.vec);
            if score.is_finite() && score > 0.05 {
                hits.push(VectorHit {
                    source_file: rec.source_file,
                    snippet: rec.snippet,
                    score,
                });
            }
        }
        hits.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        hits.truncate(limit);
        hits
    })
    .await
    .context("vector scan join")?;
    Ok(scored)
}

#[cfg(all(test, feature = "knowledge-embeddings"))]
mod tests {
    use super::*;

    #[test]
    fn merge_prefers_higher_combined_score() {
        #[derive(Clone)]
        struct Hit {
            src: String,
            snip: String,
            score: f32,
        }
        let bm25 = vec![Hit {
            src: "a.md".into(),
            snip: "hello world".into(),
            score: 2.0,
        }];
        let vec = vec![VectorHit {
            source_file: "b.md".into(),
            snippet: "other doc".into(),
            score: 0.9,
        }];
        let merged = merge_hybrid_knowledge_hits(
            bm25,
            vec,
            2,
            |h| h.src.as_str(),
            |h| h.snip.as_str(),
            |h| h.score,
            |src, snip, score| Hit { src, snip, score },
        );
        assert_eq!(merged.len(), 2);
        assert!(merged[0].score >= merged[1].score);
    }
}
