//! Read project knowledge chunks exported to ~/.anycode/knowledge/<project_id>/chunks.jsonl

use anyhow::Result;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub struct KnowledgeHit {
    pub source_file: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Deserialize)]
struct ChunkLine {
    source_file: String,
    content: String,
}

pub fn project_id_for_root(root_path: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(root_path.as_bytes());
    let digest = hasher.finalize();
    let hex = digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    format!("proj_{}", &hex[..32])
}

pub fn search_chunks_file(
    project_root: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<KnowledgeHit>> {
    let root = normalize_root(project_root)?;
    let id = project_id_for_root(&root.to_string_lossy());
    let path =
        dirs::home_dir().map(|h| h.join(".anycode/knowledge").join(&id).join("chunks.jsonl"));
    let Some(path) = path else {
        return Ok(vec![]);
    };
    if !path.is_file() {
        return Ok(vec![]);
    }
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(&path)?;
    let mut hits = Vec::new();
    for line in raw.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(chunk) = serde_json::from_str::<ChunkLine>(line) else {
            continue;
        };
        let score = crate::knowledge_scoring::score_knowledge_chunk(&q, &chunk.content);
        if score > 0.0 {
            hits.push(KnowledgeHit {
                source_file: chunk.source_file,
                snippet: chunk.content.chars().take(400).collect(),
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
    Ok(hits)
}

/// BM25 from chunks.jsonl, optionally merged with local embedding vectors when feature enabled.
pub async fn search_chunks_hybrid(
    project_root: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<KnowledgeHit>> {
    let mut hits = search_chunks_file(project_root, query, limit.saturating_mul(2))?;
    if !crate::knowledge_vectors::vectors_feature_enabled() {
        hits.truncate(limit);
        return Ok(hits);
    }
    let root = normalize_root(project_root)?;
    let id = project_id_for_root(&root.to_string_lossy());
    let vec_hits =
        crate::knowledge_vectors::search_project_vectors(&id, query, limit.saturating_mul(2))
            .await
            .unwrap_or_default();
    if vec_hits.is_empty() {
        hits.truncate(limit);
        return Ok(hits);
    }
    Ok(crate::knowledge_vectors::merge_hybrid_knowledge_hits(
        hits,
        vec_hits,
        limit,
        |h| h.source_file.as_str(),
        |h| h.snippet.as_str(),
        |h| h.score,
        |source_file, snippet, score| KnowledgeHit {
            source_file,
            snippet,
            score,
        },
    ))
}

fn normalize_root(root: &Path) -> Result<PathBuf> {
    if root.is_dir() {
        return Ok(fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf()));
    }
    Ok(root.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_id_stable() {
        assert_eq!(project_id_for_root("/tmp/x"), project_id_for_root("/tmp/x"));
    }
}
