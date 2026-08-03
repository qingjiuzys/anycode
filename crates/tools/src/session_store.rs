//! Session-scoped persistence hooks for plan trees and todos.
//!
//! Dashboard injects DB-backed implementations; headless/CLI uses in-memory only.

use crate::services::TodoItem;
use anycode_core::PlanTree;
use async_trait::async_trait;

/// Fallback key when no dashboard session is in scope (CLI / headless).
pub const EPHEMERAL_SESSION_KEY: &str = "__ephemeral__";

/// Resolve the session partition key for plan/todo state.
pub fn resolve_session_key(explicit: Option<&str>) -> String {
    if let Some(id) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return id.to_string();
    }
    anycode_core::current_dashboard_session_id()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| EPHEMERAL_SESSION_KEY.to_string())
}

#[async_trait]
pub trait SessionPlanStore: Send + Sync {
    async fn load(&self, session_id: &str) -> anyhow::Result<Option<PlanTree>>;
    async fn save(&self, session_id: &str, tree: &PlanTree) -> anyhow::Result<()>;
    async fn clear(&self, session_id: &str) -> anyhow::Result<()>;
}

#[async_trait]
pub trait SessionTodoStore: Send + Sync {
    async fn load(&self, session_id: &str) -> anyhow::Result<Option<Vec<TodoItem>>>;
    async fn save(&self, session_id: &str, todos: &[TodoItem]) -> anyhow::Result<()>;
    async fn clear(&self, session_id: &str) -> anyhow::Result<()>;
}
