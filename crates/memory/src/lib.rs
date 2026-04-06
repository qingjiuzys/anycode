//! anyCode Memory System
//!
//! 四类记忆（Project / User / Session 等）与存储后端

pub mod retrieval;

use anycode_core::prelude::*;
use async_trait::async_trait;
use moka::future::Cache;
use sled::{Db, Tree};
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;
use tracing::{debug, info};
use uuid::Uuid;
use crate::retrieval::{KeywordRetrieval, MemoryRetrieval};

// ============================================================================
// Memory Store Implementation
// ============================================================================

#[derive(Error, Debug)]
pub enum MemoryError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sled::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Memory not found: {0}")]
    NotFound(String),

    #[error("YAML parse error: {0}")]
    YamlParse(String),
}

fn core_from_mem(e: MemoryError) -> CoreError {
    CoreError::Other(anyhow::anyhow!(e))
}

fn core_from_sled(e: sled::Error) -> CoreError {
    CoreError::Other(anyhow::anyhow!(e))
}

/// 文件系统记忆存储（Markdown 树等）
pub struct FileMemoryStore {
    base_path: PathBuf,
    cache: Arc<Cache<String, Memory>>,
}

impl FileMemoryStore {
    pub fn new(base_path: impl Into<PathBuf>) -> Result<Self, MemoryError> {
        let base_path = base_path.into();

        // 创建目录结构
        std::fs::create_dir_all(base_path.join("user"))?;
        std::fs::create_dir_all(base_path.join("feedback"))?;
        std::fs::create_dir_all(base_path.join("project"))?;
        std::fs::create_dir_all(base_path.join("reference"))?;

        // 创建内存缓存 (最大 1000 条，10 分钟 TTL)
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(std::time::Duration::from_secs(600))
            .build();

        Ok(Self {
            base_path,
            cache: Arc::new(cache),
        })
    }

    fn get_type_path(&self, mem_type: &MemoryType) -> PathBuf {
        match mem_type {
            MemoryType::User => self.base_path.join("user"),
            MemoryType::Feedback => self.base_path.join("feedback"),
            MemoryType::Project => self.base_path.join("project"),
            MemoryType::Reference => self.base_path.join("reference"),
        }
    }

    async fn load_memory(&self, id: &str, mem_type: &MemoryType) -> Result<Memory, MemoryError> {
        let path = self.get_type_path(mem_type).join(format!("{}.md", id));

        // 先查缓存
        if let Some(mem) = self.cache.get(&id.to_string()).await {
            return Ok(mem);
        }

        // 读取文件
        let content = tokio::fs::read_to_string(&path).await?;
        let memory = Self::parse_markdown(&content)?;

        // 更新缓存
        self.cache.insert(id.to_string(), memory.clone()).await;

        Ok(memory)
    }

    fn parse_markdown(content: &str) -> Result<Memory, MemoryError> {
        let content = content.trim_start();
        let rest = content
            .strip_prefix("---")
            .ok_or_else(|| MemoryError::NotFound("missing frontmatter".to_string()))?;
        let rest = rest.strip_prefix('\n').unwrap_or(rest);
        let end_idx = rest
            .find("\n---\n")
            .ok_or_else(|| MemoryError::NotFound("invalid frontmatter close".to_string()))?;
        let frontmatter = &rest[..end_idx];
        let body = rest[end_idx + 5..].trim_start();

        let meta: serde_json::Value =
            serde_yaml::from_str(frontmatter).map_err(|e| MemoryError::YamlParse(e.to_string()))?;

        let id = meta
            .get("id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let mem_type = match meta.get("type").and_then(|v| v.as_str()) {
            Some("user") => MemoryType::User,
            Some("feedback") => MemoryType::Feedback,
            Some("project") => MemoryType::Project,
            Some("reference") => MemoryType::Reference,
            _ => MemoryType::User,
        };

        let title = meta
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string();

        let tags = meta
            .get("tags")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect()
            })
            .unwrap_or_default();

        let scope = match meta.get("scope").and_then(|v| v.as_str()) {
            Some("private") => MemoryScope::Private,
            Some("team") => MemoryScope::Team,
            Some("project") => MemoryScope::Project,
            _ => MemoryScope::Private,
        };

        let created_at = meta
            .get("created_at")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        let updated_at = meta
            .get("updated_at")
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .unwrap_or_else(chrono::Utc::now);

        Ok(Memory {
            id,
            mem_type,
            title,
            content: body.trim().to_string(),
            tags,
            scope,
            created_at,
            updated_at,
        })
    }
}

#[async_trait]
impl MemoryStore for FileMemoryStore {
    async fn save(&self, memory: Memory) -> Result<(), CoreError> {
        let type_path = self.get_type_path(&memory.mem_type);
        let file_path = type_path.join(format!("{}.md", memory.id));

        // 构建 Markdown 内容
        let mut content = String::new();
        content.push_str("---\n");
        content.push_str(&format!("id: {}\n", memory.id));
        let type_s = match memory.mem_type {
            MemoryType::User => "user",
            MemoryType::Feedback => "feedback",
            MemoryType::Project => "project",
            MemoryType::Reference => "reference",
        };
        content.push_str(&format!("type: {}\n", type_s));
        content.push_str(&format!("title: {}\n", memory.title));
        if !memory.tags.is_empty() {
            content.push_str(&format!("tags: {:?}\n", memory.tags));
        }
        let scope_s = match memory.scope {
            MemoryScope::Private => "private",
            MemoryScope::Team => "team",
            MemoryScope::Project => "project",
        };
        content.push_str(&format!("scope: {}\n", scope_s));
        content.push_str(&format!("created_at: {}\n", memory.created_at.to_rfc3339()));
        content.push_str(&format!("updated_at: {}\n", memory.updated_at.to_rfc3339()));
        content.push_str("---\n\n");
        content.push_str(&memory.content);

        // 写入文件
        tokio::fs::write(&file_path, content).await?;

        // 更新缓存
        self.cache.insert(memory.id.clone(), memory.clone()).await;

        // 更新索引
        self.update_index(&memory).await?;

        info!("Memory saved: {} ({})", memory.id, memory.title);
        Ok(())
    }

    async fn recall(&self, query: &str, mem_type: MemoryType) -> Result<Vec<Memory>, CoreError> {
        let type_path = self.get_type_path(&mem_type);
        let mut memories = Vec::new();

        // 读取该类型的所有记忆
        let mut entries = tokio::fs::read_dir(type_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            match self
                .load_memory(
                    path.file_stem().and_then(|s| s.to_str()).unwrap_or(""),
                    &mem_type,
                )
                .await
            {
                Ok(memory) => {
                    // 简单的搜索匹配
                    if memory.content.contains(query)
                        || memory.title.contains(query)
                        || memory.tags.iter().any(|t| t.contains(query))
                    {
                        memories.push(memory);
                    }
                }
                Err(e) => {
                    debug!("Failed to load memory: {:?}", e);
                }
            }
        }

        Ok(KeywordRetrieval.rank(query, memories))
    }

    async fn update(&self, _id: &str, memory: Memory) -> Result<(), CoreError> {
        let mut updated_memory = memory;
        updated_memory.updated_at = chrono::Utc::now();

        self.save(updated_memory).await
    }

    async fn delete(&self, id: &str) -> Result<(), CoreError> {
        // 需要先找到文件
        for mem_type in &[
            MemoryType::User,
            MemoryType::Feedback,
            MemoryType::Project,
            MemoryType::Reference,
        ] {
            let path = self.get_type_path(mem_type).join(format!("{}.md", id));
            if path.exists() {
                tokio::fs::remove_file(&path).await?;

                // 从缓存移除
                self.cache.invalidate(&id.to_string()).await;

                return Ok(());
            }
        }

        Err(CoreError::Other(anyhow::anyhow!(
            "Memory not found: {}",
            id
        )))
    }
}

impl FileMemoryStore {
    async fn update_index(&self, memory: &Memory) -> Result<(), CoreError> {
        let index_path = self.base_path.join("MEMORY.md");

        // 读取现有索引
        let existing_content = if index_path.exists() {
            tokio::fs::read_to_string(&index_path).await?
        } else {
            String::from("# Memory Index\n\n")
        };

        // 检查是否已存在
        let entry = format!(
            "- [{}]({}.md) — {}",
            memory.title,
            memory.id,
            memory.tags.join(", ")
        );

        if !existing_content.contains(&format!("({}.md)", memory.id)) {
            // 添加到索引
            let mut new_content = existing_content;
            new_content.push_str(&entry);
            new_content.push_str("\n");
            tokio::fs::write(&index_path, new_content).await?;
        }

        Ok(())
    }
}

// ============================================================================
// Sled Memory Store (可选)
// ============================================================================

pub struct SledMemoryStore {
    db: Db,
    cache: Arc<Cache<String, Memory>>,
}

impl SledMemoryStore {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, MemoryError> {
        let path: PathBuf = path.into();
        let db = sled::open(path.as_path())?;

        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(std::time::Duration::from_secs(600))
            .build();

        Ok(Self {
            db,
            cache: Arc::new(cache),
        })
    }

    fn get_tree(&self, mem_type: &MemoryType) -> Result<Tree, MemoryError> {
        let tree_name = match mem_type {
            MemoryType::User => "user",
            MemoryType::Feedback => "feedback",
            MemoryType::Project => "project",
            MemoryType::Reference => "reference",
        };
        Ok(self.db.open_tree(tree_name)?)
    }
}

#[async_trait]
impl MemoryStore for SledMemoryStore {
    async fn save(&self, memory: Memory) -> Result<(), CoreError> {
        let tree = self.get_tree(&memory.mem_type).map_err(core_from_mem)?;
        let key = memory.id.as_bytes();
        let value = serde_json::to_vec(&memory)?;
        tree.insert(key, value).map_err(core_from_sled)?;
        tree.flush().map_err(core_from_sled)?;

        self.cache.insert(memory.id.clone(), memory).await;
        Ok(())
    }

    async fn recall(&self, query: &str, mem_type: MemoryType) -> Result<Vec<Memory>, CoreError> {
        let tree = self.get_tree(&mem_type).map_err(core_from_mem)?;
        let mut memories = Vec::new();

        for result in tree.iter() {
            let (_, value) = result.map_err(core_from_sled)?;
            match serde_json::from_slice::<Memory>(&value) {
                Ok(memory) => {
                    if memory.content.contains(query)
                        || memory.title.contains(query)
                        || memory.tags.iter().any(|t| t.contains(query))
                    {
                        memories.push(memory);
                    }
                }
                Err(e) => {
                    debug!("Failed to deserialize memory: {:?}", e);
                }
            }
        }

        Ok(KeywordRetrieval.rank(query, memories))
    }

    async fn update(&self, _id: &str, memory: Memory) -> Result<(), CoreError> {
        self.save(memory).await
    }

    async fn delete(&self, id: &str) -> Result<(), CoreError> {
        for mem_type in [
            MemoryType::User,
            MemoryType::Feedback,
            MemoryType::Project,
            MemoryType::Reference,
        ] {
            let tree = match self.get_tree(&mem_type) {
                Ok(t) => t,
                Err(_) => continue,
            };
            if tree
                .remove(id.as_bytes())
                .map_err(core_from_sled)?
                .is_some()
            {
                tree.flush().map_err(core_from_sled)?;
                self.cache.invalidate(&id.to_string()).await;
                return Ok(());
            }
        }
        Ok(())
    }
}

// ============================================================================
// Hybrid Memory Store (最佳性能)
// ============================================================================

pub struct HybridMemoryStore {
    sled: SledMemoryStore,
    file: FileMemoryStore,
}

impl HybridMemoryStore {
    pub fn new(
        sled_path: impl Into<PathBuf>,
        file_path: impl Into<PathBuf>,
    ) -> Result<Self, MemoryError> {
        Ok(Self {
            sled: SledMemoryStore::new(sled_path)?,
            file: FileMemoryStore::new(file_path)?,
        })
    }
}

#[async_trait]
impl MemoryStore for HybridMemoryStore {
    async fn save(&self, memory: Memory) -> Result<(), CoreError> {
        // 同时保存到两个存储
        self.sled.save(memory.clone()).await?;
        self.file.save(memory).await?;
        Ok(())
    }

    async fn recall(&self, query: &str, mem_type: MemoryType) -> Result<Vec<Memory>, CoreError> {
        // 优先从 sled 读取（更快）
        self.sled.recall(query, mem_type).await
    }

    async fn update(&self, id: &str, memory: Memory) -> Result<(), CoreError> {
        self.sled.update(id, memory.clone()).await?;
        self.file.update(id, memory).await
    }

    async fn delete(&self, id: &str) -> Result<(), CoreError> {
        self.sled.delete(id).await?;
        self.file.delete(id).await
    }
}
