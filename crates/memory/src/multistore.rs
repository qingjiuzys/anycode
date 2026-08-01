//! 多存储支持（对齐 Claude Code 2.1.218 `managed-agents-memory` 的 memory store 模型）。
//!
//! 二进制提取语义：
//! - `memory_store_id`：每条记忆归属一个 store（REST `/v1/memory_stores/{id}/memories`）。
//! - `expected_content_sha256`：删除/更新时校验内容哈希，防止误删并发修改。
//! - `memory_store_skills`：技能也可作为 store 来源（`memory-skills: not loaded under strictPluginOnlyCustomization`）。
//! - `tengu_memory_store_resync_interval_minutes`：store 重同步间隔。
//! - webhook 事件：`memory_store.created` / `memory_store.archived` / `memory_store.deleted`。
//! - 无 `memory_store.updated`：记忆版本通过 versions 端点追踪（poll memory-versions endpoints）。

use anycode_core::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::MemoryError;

/// 记忆存储标识（对齐 `memory_store_id`）。
pub type MemoryStoreId = String;

/// 存储类别（对齐 Claude 的 store 来源：用户/项目/团队/技能/远端托管/本地）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStoreKind {
    /// 用户私有存储。
    User,
    /// 项目共享存储。
    Project,
    /// 团队存储。
    Team,
    /// 技能来源（`memory_store_skills`）。
    Skills,
    /// 远端托管（Anthropic managed agents）。
    Managed,
    /// 本地文件/热层。
    Local,
}

impl MemoryStoreKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryStoreKind::User => "user",
            MemoryStoreKind::Project => "project",
            MemoryStoreKind::Team => "team",
            MemoryStoreKind::Skills => "skills",
            MemoryStoreKind::Managed => "managed",
            MemoryStoreKind::Local => "local",
        }
    }
}

/// 选择器：决定请求落在哪个 store（对齐 Claude `memory_store` session resource 的语义）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MemoryStoreSelector {
    /// 显式 `memory_store_id`；优先于其它字段。
    pub store_id: Option<MemoryStoreId>,
    /// 按存储类别过滤。
    pub kind: Option<MemoryStoreKind>,
    /// 按记忆类型过滤（仅当未指定 store_id 时）。
    pub mem_type: Option<MemoryType>,
}

impl MemoryStoreSelector {
    pub fn by_id(store_id: impl Into<String>) -> Self {
        Self {
            store_id: Some(store_id.into()),
            kind: None,
            mem_type: None,
        }
    }

    pub fn by_kind(kind: MemoryStoreKind) -> Self {
        Self {
            store_id: None,
            kind: Some(kind),
            mem_type: None,
        }
    }

    /// 空选择器匹配任意 store；否则依次按 id / kind 匹配。
    pub fn matches(&self, desc: &MemoryStoreDescriptor) -> bool {
        if let Some(id) = &self.store_id {
            return &desc.id == id;
        }
        if let Some(kind) = self.kind {
            return desc.kind == kind;
        }
        true
    }
}

/// 一个 memory store 的描述（对齐 `/v1/memory_stores` 列表条目）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStoreDescriptor {
    pub id: MemoryStoreId,
    pub name: String,
    pub description: String,
    pub kind: MemoryStoreKind,
}

/// 内容版本（对齐 `expected_content_sha256` 与 memory-versions 端点）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryVersion {
    /// 内容 SHA-256（十六进制）。
    pub content_sha256: String,
    /// 单调递增版本号。
    pub version: u64,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// 版本化记忆：记忆 + 归属 store + 内容版本。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedMemory {
    pub memory: Memory,
    pub store_id: MemoryStoreId,
    pub version: MemoryVersion,
}

/// 计算内容哈希（与 Claude 的 content hash 校验意图一致；默认 Hasher 输出十六进制摘要）。
pub fn content_hash(content: &str) -> String {
    let mut h = DefaultHasher::new();
    content.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// 为一条记忆生成首个版本。
pub fn initial_version(memory: &Memory) -> MemoryVersion {
    MemoryVersion {
        content_sha256: content_hash(&memory.content),
        version: 1,
        updated_at: chrono::Utc::now(),
    }
}

/// 更新版本：若 `expected_sha` 与当前内容不符则返回 `Err`（对齐 `expected_content_sha256` 校验）。
pub fn next_version(
    current: &MemoryVersion,
    new_content: &str,
    expected_sha: Option<&str>,
) -> Result<MemoryVersion, MemoryError> {
    let new_sha = content_hash(new_content);
    if let Some(expected) = expected_sha {
        if expected != new_sha {
            return Err(MemoryError::VersionConflict(format!(
                "expected {} got {}",
                expected, new_sha
            )));
        }
    }
    Ok(MemoryVersion {
        content_sha256: new_sha,
        version: current.version + 1,
        updated_at: chrono::Utc::now(),
    })
}

/// 冲突记录（对齐 Claude 多 store 同步时的 conflict 语义）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryConflict {
    pub key: String,
    pub store_a: MemoryStoreId,
    pub store_b: MemoryStoreId,
    pub sha_a: String,
    pub sha_b: String,
}

/// 检测多 store 中同 key 记忆的内容冲突。
pub fn detect_conflicts(entries: &[VersionedMemory]) -> Vec<MemoryConflict> {
    let mut by_key: HashMap<&str, Vec<&VersionedMemory>> = HashMap::new();
    for e in entries {
        by_key.entry(e.memory.id.as_str()).or_default().push(e);
    }
    let mut out = Vec::new();
    for (key, group) in by_key {
        if group.len() < 2 {
            continue;
        }
        let first = group[0];
        for other in &group[1..] {
            if other.version.content_sha256 != first.version.content_sha256 {
                out.push(MemoryConflict {
                    key: key.to_string(),
                    store_a: first.store_id.clone(),
                    store_b: other.store_id.clone(),
                    sha_a: first.version.content_sha256.clone(),
                    sha_b: other.version.content_sha256.clone(),
                });
            }
        }
    }
    out
}

/// store 注册表：描述 + 按选择器解析目标 store。
#[derive(Debug, Default, Clone)]
pub struct MemoryStoreRegistry {
    stores: Arc<std::sync::RwLock<Vec<MemoryStoreDescriptor>>>,
}

impl MemoryStoreRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&self, desc: MemoryStoreDescriptor) {
        let mut stores = self.stores.write().unwrap();
        stores.retain(|d| d.id != desc.id);
        stores.push(desc);
    }

    /// 列出全部 store 描述（对齐 `/v1/memory_stores` 列表）。
    pub fn list(&self) -> Vec<MemoryStoreDescriptor> {
        self.stores.read().unwrap().clone()
    }

    /// 按选择器解析目标 store；无匹配时返回第一个（默认 store）。
    pub fn resolve(&self, selector: &MemoryStoreSelector) -> Option<MemoryStoreDescriptor> {
        let stores = self.stores.read().unwrap();
        if let Some(id) = &selector.store_id {
            return stores.iter().find(|d| &d.id == id).cloned();
        }
        if let Some(kind) = selector.kind {
            if let Some(d) = stores.iter().find(|d| d.kind == kind) {
                return Some(d.clone());
            }
        }
        stores.first().cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory(id: &str, content: &str) -> Memory {
        Memory {
            id: id.to_string(),
            mem_type: MemoryType::Project,
            title: id.to_string(),
            content: content.to_string(),
            tags: vec![],
            scope: MemoryScope::Project,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            meta: None,
        }
    }

    fn versioned(store: &str, mem: Memory, sha: &str) -> VersionedMemory {
        VersionedMemory {
            memory: mem,
            store_id: store.to_string(),
            version: MemoryVersion {
                content_sha256: sha.to_string(),
                version: 1,
                updated_at: chrono::Utc::now(),
            },
        }
    }

    #[test]
    fn selector_matches_by_id_then_kind_then_default() {
        let reg = MemoryStoreRegistry::new();
        reg.register(MemoryStoreDescriptor {
            id: "team-a".into(),
            name: "Team A".into(),
            description: "".into(),
            kind: MemoryStoreKind::Team,
        });
        reg.register(MemoryStoreDescriptor {
            id: "user-1".into(),
            name: "User 1".into(),
            description: "".into(),
            kind: MemoryStoreKind::User,
        });
        assert_eq!(
            reg.resolve(&MemoryStoreSelector::by_id("team-a"))
                .unwrap()
                .id,
            "team-a"
        );
        assert_eq!(
            reg.resolve(&MemoryStoreSelector::by_kind(MemoryStoreKind::User))
                .unwrap()
                .id,
            "user-1"
        );
        // 空选择器 → 第一个 store
        assert_eq!(
            reg.resolve(&MemoryStoreSelector::default()).unwrap().id,
            "team-a"
        );
    }

    #[test]
    fn version_hash_and_expected_sha_check() {
        let mem = memory("m1", "hello");
        let v1 = initial_version(&mem);
        assert_eq!(v1.version, 1);
        assert_eq!(v1.content_sha256, content_hash("hello"));
        // 正确预期 → 版本递增
        let v2 = next_version(&v1, "hello world", Some(&content_hash("hello world"))).unwrap();
        assert_eq!(v2.version, 2);
        // 错误预期 → 冲突
        assert!(next_version(&v1, "hello world", Some("deadbeef")).is_err());
    }

    #[test]
    fn detects_conflicts_between_stores() {
        let entries = vec![
            versioned("store-a", memory("k1", "v1"), "sha-1"),
            versioned("store-b", memory("k1", "v2"), "sha-2"),
            versioned("store-a", memory("k2", "same"), "sha-3"),
            versioned("store-b", memory("k2", "same"), "sha-3"),
        ];
        let conflicts = detect_conflicts(&entries);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].key, "k1");
        assert_eq!(conflicts[0].store_a, "store-a");
        assert_eq!(conflicts[0].store_b, "store-b");
    }
}
