//! Project-scoped knowledge base: index text files under configured paths and search chunks.

use crate::db::DashboardDb;
use crate::project_root::{normalize_project_root, project_id_for_root};
use anyhow::{Context, Result};
use sqlx::Row;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_FILE_BYTES: u64 = 512 * 1024;
const CHUNK_CHARS: usize = 2000;

#[derive(Debug, Clone, serde::Serialize)]
pub struct KnowledgeSearchHit {
    pub source_file: String,
    pub snippet: String,
    pub score: f32,
}

pub async fn list_paths(db: &DashboardDb, project_id: &str) -> Result<Vec<String>> {
    let rows = sqlx::query_scalar::<_, String>(
        "SELECT rel_path FROM project_knowledge_paths WHERE project_id = ? ORDER BY rel_path",
    )
    .bind(project_id)
    .fetch_all(db.pool())
    .await?;
    Ok(rows)
}

pub async fn set_paths(db: &DashboardDb, project_id: &str, paths: &[String]) -> Result<()> {
    sqlx::query("DELETE FROM project_knowledge_paths WHERE project_id = ?")
        .bind(project_id)
        .execute(db.pool())
        .await?;
    for p in paths {
        let p = p.trim();
        if p.is_empty() {
            continue;
        }
        sqlx::query("INSERT INTO project_knowledge_paths (project_id, rel_path) VALUES (?, ?)")
            .bind(project_id)
            .bind(p)
            .execute(db.pool())
            .await?;
    }
    Ok(())
}

pub async fn reindex_project(db: &DashboardDb, project_root: &Path) -> Result<usize> {
    let root = normalize_project_root(project_root)?;
    let project_id = project_id_for_root(&root.to_string_lossy());
    let paths = list_paths(db, &project_id).await?;
    sqlx::query("DELETE FROM project_knowledge_chunks WHERE project_id = ?")
        .bind(&project_id)
        .execute(db.pool())
        .await?;
    let mut count = 0usize;
    for rel in paths {
        let abs = root.join(rel.trim_start_matches('/'));
        if abs.is_file() {
            if let Some(n) = index_file(db, &project_id, &root, &abs, &rel).await? {
                count += n;
            }
        } else if abs.is_dir() {
            count += index_dir(db, &project_id, &root, &abs, &rel).await?;
        }
    }
    export_chunks_jsonl(db, &project_id).await?;
    rebuild_vectors(db, &project_id).await?;
    Ok(count)
}

async fn rebuild_vectors(db: &DashboardDb, project_id: &str) -> Result<()> {
    if !anycode_tools::vectors_feature_enabled() {
        return Ok(());
    }
    let rows = sqlx::query(
        "SELECT id, source_file, content FROM project_knowledge_chunks WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(db.pool())
    .await?;
    let mut tuples = Vec::with_capacity(rows.len());
    for r in rows {
        let id: String = r.get("id");
        let source: String = r.get("source_file");
        let content: String = r.get("content");
        tuples.push((id, source, content));
    }
    let _ = anycode_tools::rebuild_project_vectors(project_id, &tuples).await?;
    Ok(())
}

fn knowledge_cache_dir(project_id: &str) -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".anycode/knowledge").join(project_id))
}

async fn export_chunks_jsonl(db: &DashboardDb, project_id: &str) -> Result<()> {
    let Some(dir) = knowledge_cache_dir(project_id) else {
        return Ok(());
    };
    fs::create_dir_all(&dir)?;
    let rows = sqlx::query(
        "SELECT source_file, content FROM project_knowledge_chunks WHERE project_id = ?",
    )
    .bind(project_id)
    .fetch_all(db.pool())
    .await?;
    let path = dir.join("chunks.jsonl");
    let mut out = String::new();
    for r in rows {
        let source: String = r.get("source_file");
        let content: String = r.get("content");
        let line = serde_json::json!({ "source_file": source, "content": content });
        out.push_str(&line.to_string());
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}

async fn index_dir(
    db: &DashboardDb,
    project_id: &str,
    root: &Path,
    dir: &Path,
    rel_prefix: &str,
) -> Result<usize> {
    let mut total = 0usize;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for ent in fs::read_dir(&d).with_context(|| format!("read_dir {}", d.display()))? {
            let ent = ent?;
            let p = ent.path();
            if p.is_dir() {
                if should_skip_dir(&p) {
                    continue;
                }
                stack.push(p);
            } else if p.is_file() && is_text_candidate(&p) {
                let rel = rel_path_from_root(root, &p);
                if let Some(n) = index_file(db, project_id, root, &p, &rel).await? {
                    total += n;
                }
            }
        }
    }
    let _ = rel_prefix;
    Ok(total)
}

async fn index_file(
    db: &DashboardDb,
    project_id: &str,
    root: &Path,
    file: &Path,
    rel: &str,
) -> Result<Option<usize>> {
    let meta = fs::metadata(file)?;
    if meta.len() > MAX_FILE_BYTES {
        return Ok(None);
    }
    let text = fs::read_to_string(file).unwrap_or_default();
    if text.trim().is_empty() {
        return Ok(None);
    }
    let rel_display = rel_path_from_root(root, file);
    let chunks = chunk_text(&text, CHUNK_CHARS);
    for (i, chunk) in chunks.iter().enumerate() {
        let id = Uuid::new_v4().to_string();
        sqlx::query(
            r#"
            INSERT INTO project_knowledge_chunks
              (id, project_id, rel_path, source_file, chunk_index, content)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(rel)
        .bind(&rel_display)
        .bind(i as i64)
        .bind(chunk)
        .execute(db.pool())
        .await?;
    }
    Ok(Some(chunks.len()))
}

pub async fn search(
    db: &DashboardDb,
    project_root: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<KnowledgeSearchHit>> {
    let root = normalize_project_root(project_root)?;
    let project_id = project_id_for_root(&root.to_string_lossy());
    let q = query.trim();
    if q.is_empty() {
        return Ok(vec![]);
    }
    let rows = sqlx::query(
        "SELECT source_file, content FROM project_knowledge_chunks WHERE project_id = ?",
    )
    .bind(&project_id)
    .fetch_all(db.pool())
    .await?;
    let mut hits: Vec<KnowledgeSearchHit> = Vec::new();
    for r in rows {
        let source: String = r.get("source_file");
        let content: String = r.get("content");
        let score = anycode_tools::score_knowledge_chunk(&q, &content);
        if score > 0.0 {
            let snippet: String = content.chars().take(400).collect();
            hits.push(KnowledgeSearchHit {
                source_file: source,
                snippet,
                score,
            });
        }
    }
    hits.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if anycode_tools::vectors_feature_enabled() {
        if let Ok(vec_hits) =
            anycode_tools::search_project_vectors(&project_id, q, limit.saturating_mul(2)).await
        {
            if !vec_hits.is_empty() {
                hits = anycode_tools::merge_hybrid_knowledge_hits(
                    hits,
                    vec_hits,
                    limit,
                    |h| h.source_file.as_str(),
                    |h| h.snippet.as_str(),
                    |h| h.score,
                    |source_file, snippet, score| KnowledgeSearchHit {
                        source_file,
                        snippet,
                        score,
                    },
                );
                return Ok(hits);
            }
        }
    }
    hits.truncate(limit);
    Ok(hits)
}

/// Search by cwd when projects.db exists.
pub async fn search_for_cwd(
    cwd: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<KnowledgeSearchHit>> {
    let path = crate::server::default_db_path();
    if !path.is_file() {
        return Ok(vec![]);
    }
    let db = DashboardDb::open(&path).await?;
    search(&db, cwd, query, limit).await
}

fn chunk_text(text: &str, size: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut start = 0;
    let chars: Vec<char> = text.chars().collect();
    while start < chars.len() {
        let end = (start + size).min(chars.len());
        out.push(chars[start..end].iter().collect());
        start = end;
    }
    out
}

fn rel_path_from_root(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| file.display().to_string())
}

fn is_text_candidate(p: &Path) -> bool {
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "md" | "txt"
            | "rst"
            | "json"
            | "yaml"
            | "yml"
            | "toml"
            | "rs"
            | "py"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "csv"
    ) || ext.is_empty()
}

fn should_skip_dir(p: &Path) -> bool {
    let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
    matches!(
        name,
        ".git" | "node_modules" | "target" | "dist" | "build" | ".anycode"
    )
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct KnowledgeStats {
    pub path_count: i64,
    pub chunk_count: i64,
    pub cache_path: Option<String>,
    pub cache_bytes: Option<u64>,
    pub vectors_enabled: bool,
    pub vector_count: usize,
    pub vector_store_path: Option<String>,
}

pub async fn stats(db: &DashboardDb, project_id: &str) -> Result<KnowledgeStats> {
    let path_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_knowledge_paths WHERE project_id = ?")
            .bind(project_id)
            .fetch_one(db.pool())
            .await?;
    let chunk_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM project_knowledge_chunks WHERE project_id = ?")
            .bind(project_id)
            .fetch_one(db.pool())
            .await?;
    let cache_file = knowledge_cache_dir(project_id).map(|d| d.join("chunks.jsonl"));
    let (cache_path, cache_bytes) = match cache_file {
        Some(p) if p.is_file() => {
            let bytes = fs::metadata(&p).ok().map(|m| m.len());
            (Some(p.display().to_string()), bytes)
        }
        Some(p) => (Some(p.display().to_string()), None),
        None => (None, None),
    };
    Ok(KnowledgeStats {
        path_count,
        chunk_count,
        cache_path,
        cache_bytes,
        vectors_enabled: anycode_tools::vectors_feature_enabled(),
        vector_count: anycode_tools::vector_chunk_count(project_id),
        vector_store_path: anycode_tools::vector_store_path(project_id)
            .map(|p| p.display().to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_text_splits() {
        let c = chunk_text("abcdef", 2);
        assert_eq!(c, vec!["ab", "cd", "ef"]);
    }
}
