//! 记忆领域模型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// 记忆类型
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryType {
    User,
    Feedback,
    Project,
    Reference,
}

impl MemoryType {
    pub const ALL: [MemoryType; 4] = [
        MemoryType::User,
        MemoryType::Feedback,
        MemoryType::Project,
        MemoryType::Reference,
    ];

    /// Stable storage label used by file/sled/vector backends.
    pub fn as_storage_str(self) -> &'static str {
        match self {
            MemoryType::User => "user",
            MemoryType::Feedback => "feedback",
            MemoryType::Project => "project",
            MemoryType::Reference => "reference",
        }
    }

    /// Reverse mapping for on-disk labels.
    pub fn from_storage_str(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "user" => Some(MemoryType::User),
            "feedback" => Some(MemoryType::Feedback),
            "project" => Some(MemoryType::Project),
            "reference" => Some(MemoryType::Reference),
            _ => None,
        }
    }

    /// Compact discriminant for keyed stores.
    pub fn discriminant(self) -> u8 {
        match self {
            MemoryType::User => 0,
            MemoryType::Feedback => 1,
            MemoryType::Project => 2,
            MemoryType::Reference => 3,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::MemoryType;

    #[test]
    fn memory_type_storage_roundtrip_and_all() {
        assert_eq!(MemoryType::ALL.len(), 4);
        for t in MemoryType::ALL {
            let s = t.as_storage_str();
            assert_eq!(MemoryType::from_storage_str(s), Some(t));
        }
        assert_eq!(MemoryType::from_storage_str("unknown"), None);
    }
}
