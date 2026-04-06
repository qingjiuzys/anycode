//! 记忆领域模型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 记忆类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryType {
    User,
    Feedback,
    Project,
    Reference,
}

/// 记忆
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Memory {
    pub id: String,
    pub mem_type: MemoryType,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub scope: MemoryScope,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 记忆范围
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryScope {
    Private,
    Team,
    Project,
}
