//! 归根通道（分层记忆管线）领域类型与 trait。
//!
//! 与 [`crate::traits::MemoryStore`] 区分：`MemoryStore` 表示**已巩固、可直接 CRUD 的投影**（legacy file/sled/hybrid）；
//! `MemoryPipeline` 表示虚态缓冲 → 强化 → 热层/向量的管线。

use crate::error::CoreError;
use crate::memory_model::{Memory, MemoryType};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 前语义印象：尚未向量化、未入热层巩固库的原始片段。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreSemanticFragment {
    pub id: String,
    /// 会话或任务命名空间，便于按会话淘汰。
    pub session_id: String,
    pub mem_type: MemoryType,
    pub raw_text: String,
    pub created_at: DateTime<Utc>,
    pub last_touched_at: DateTime<Utc>,
    pub touch_count: u32,
}

/// 管线参数（运行时）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPipelineSettings {
    /// 虚态缓冲：条目超过该时长未触碰则淘汰（秒）。
    pub buffer_ttl_secs: u64,
    /// 缓冲内最大条数（超出则丢弃最旧）。
    pub max_buffer_fragments: usize,
    /// 触碰次数达到该值则晋升到热层。
    pub promote_touch_threshold: u32,
    /// 是否在 recall 时对与 query 匹配的缓冲条目计为一次触碰。
    pub reinforce_on_recall_match: bool,
    /// 是否合并只读 legacy Markdown 目录的 recall（与 pipeline 热层并行）。
    pub merge_legacy_file_recall: bool,
    /// 是否将虚态缓冲追加写入 WAL（崩溃恢复）。
    pub buffer_wal_enabled: bool,
    /// 每 N 条 WAL 记录后 `fsync`（1 = 每条刷盘）。
    pub buffer_wal_fsync_every_n: u32,
    /// 工具结果回注后是否 ingest（需 `memory_pipeline` 存在）。
    pub hook_after_tool_result: bool,
    /// 本轮 assistant 结束且无后续 tool_calls 时是否 ingest 摘要。
    pub hook_after_agent_turn: bool,
    /// 钩子写入片段的最大字节数。
    pub hook_max_bytes: usize,
    /// 工具名前缀命中则跳过钩子（如 `mcp__`）。
    pub hook_tool_deny_prefixes: Vec<String>,
    /// 是否启用 HTTP embedding + Sled 向量索引（需配置模型与密钥）。
    pub embedding_enabled: bool,
}

impl Default for MemoryPipelineSettings {
    fn default() -> Self {
        Self {
            buffer_ttl_secs: 86_400,
            max_buffer_fragments: 256,
            promote_touch_threshold: 3,
            reinforce_on_recall_match: true,
            merge_legacy_file_recall: true,
            buffer_wal_enabled: true,
            buffer_wal_fsync_every_n: 32,
            hook_after_tool_result: true,
            hook_after_agent_turn: true,
            hook_max_bytes: 4096,
            hook_tool_deny_prefixes: vec!["mcp__".to_string()],
            embedding_enabled: false,
        }
    }
}

/// 嵌入向量提供者（可选；无则向量层跳过）。
#[async_trait]
pub trait EmbeddingProvider: Send + Sync {
    /// 单条文本嵌入；失败时上层可降级为仅关键词/热层。
    async fn embed_one(&self, text: &str) -> Result<Vec<f32>, CoreError>;
}

/// 向量索引后端（可选实现；默认 noop）。
#[async_trait]
pub trait VectorMemoryBackend: Send + Sync {
    async fn upsert(
        &self,
        id: &str,
        embedding: &[f32],
        mem_type: MemoryType,
    ) -> Result<(), CoreError>;

    /// 返回按相似度排序的 id 列表（最相似在前）。
    async fn search(
        &self,
        query_embedding: &[f32],
        mem_type: MemoryType,
        limit: usize,
    ) -> Result<Vec<String>, CoreError>;

    async fn remove(&self, id: &str, mem_type: MemoryType) -> Result<(), CoreError>;
}

/// 归根通道记忆管线。
#[async_trait]
pub trait MemoryPipeline: Send + Sync {
    /// 摄入原始片段，进入虚态缓冲；返回 fragment id。
    async fn ingest_fragment(
        &self,
        session_id: &str,
        text: &str,
        mem_type: MemoryType,
    ) -> Result<String, CoreError>;

    /// 显式强化（触碰 +1，可能触发晋升）。
    async fn touch(&self, fragment_id: &str) -> Result<(), CoreError>;

    /// 衰减：淘汰过期或未强化的缓冲条目；热层/向量层的降级策略由实现决定。
    async fn tick_decay(&self) -> Result<(), CoreError>;

    /// 合并缓冲匹配、热层关键词、向量检索（若启用），去重排序后供注入模型。
    async fn materialize_for_prompt(
        &self,
        query: &str,
        mem_type: MemoryType,
    ) -> Result<Vec<Memory>, CoreError>;

    /// 将虚态缓冲中指定 id **立即** 晋升到热层（忽略 touch 阈值）。
    async fn promote_fragment_to_hot(&self, fragment_id: &str) -> Result<(), CoreError>;

    /// 对热层已有记忆补建向量索引（若启用 embedding）。
    async fn promote_memory_to_vector(
        &self,
        memory_id: &str,
        mem_type: MemoryType,
    ) -> Result<(), CoreError>;

    /// 将虚态缓冲 WAL 等易失层刷盘。默认空操作；`pipeline` 实现会在 drop 时再次 best-effort 刷盘。
    fn sync_durability(&self) -> Result<(), CoreError> {
        Ok(())
    }
}
