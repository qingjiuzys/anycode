//! DB-backed session plan tree / todo stores wired into ToolServices.

use crate::db::DashboardDb;
use crate::events::EventBus;
use crate::schema::InsertEventRequest;
use anycode_core::PlanTree;
use anycode_tools::{SessionPlanStore, SessionTodoStore, TodoItem};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

#[derive(Clone)]
pub struct DbSessionPlanStore {
    db: DashboardDb,
    events: Arc<EventBus>,
}

impl DbSessionPlanStore {
    #[must_use]
    pub fn new(db: DashboardDb, events: Arc<EventBus>) -> Self {
        Self { db, events }
    }

    async fn emit_updated(&self, session_id: &str, tree: Option<&PlanTree>) {
        let Ok(Some(session)) = self.db.get_session(session_id).await else {
            return;
        };
        let title = if tree.map(|t| t.roots.is_empty()).unwrap_or(true) {
            "Plan tree cleared"
        } else {
            "Plan tree updated"
        };
        let payload = tree.map(|t| json!({ "tree": t }));
        match self
            .db
            .insert_event(InsertEventRequest {
                project_id: session.project_id.clone(),
                session_id: Some(session_id.to_string()),
                task_id: None,
                agent_id: None,
                event_type: "plan_tree_updated".into(),
                severity: Some("info".into()),
                title: title.into(),
                body: None,
                payload,
            })
            .await
        {
            Ok(evt) => self.events.publish(evt),
            Err(e) => {
                tracing::warn!(
                    target: "anycode_dashboard",
                    error = %e,
                    session_id,
                    "failed to emit plan_tree_updated event"
                );
            }
        }
    }
}

#[async_trait]
impl SessionPlanStore for DbSessionPlanStore {
    async fn load(&self, session_id: &str) -> anyhow::Result<Option<PlanTree>> {
        Ok(self
            .db
            .get_session_plan_tree(session_id)
            .await?
            .map(|(tree, _)| tree))
    }

    async fn save(&self, session_id: &str, tree: &PlanTree) -> anyhow::Result<()> {
        self.db.upsert_session_plan_tree(session_id, tree).await?;
        self.emit_updated(session_id, Some(tree)).await;
        Ok(())
    }

    async fn clear(&self, session_id: &str) -> anyhow::Result<()> {
        self.db.delete_session_plan_tree(session_id).await?;
        self.emit_updated(session_id, None).await;
        Ok(())
    }
}

#[derive(Clone)]
pub struct DbSessionTodoStore {
    db: DashboardDb,
    events: Arc<EventBus>,
}

impl DbSessionTodoStore {
    #[must_use]
    pub fn new(db: DashboardDb, events: Arc<EventBus>) -> Self {
        Self { db, events }
    }

    async fn emit_updated(&self, session_id: &str, todos: &[TodoItem]) {
        let Ok(Some(session)) = self.db.get_session(session_id).await else {
            return;
        };
        match self
            .db
            .insert_event(InsertEventRequest {
                project_id: session.project_id.clone(),
                session_id: Some(session_id.to_string()),
                task_id: None,
                agent_id: None,
                event_type: "session_todos_updated".into(),
                severity: Some("info".into()),
                title: "Session todos updated".into(),
                body: None,
                payload: Some(json!({ "todos": todos })),
            })
            .await
        {
            Ok(evt) => self.events.publish(evt),
            Err(e) => {
                tracing::warn!(
                    target: "anycode_dashboard",
                    error = %e,
                    session_id,
                    "failed to emit session_todos_updated event"
                );
            }
        }
    }
}

#[async_trait]
impl SessionTodoStore for DbSessionTodoStore {
    async fn load(&self, session_id: &str) -> anyhow::Result<Option<Vec<TodoItem>>> {
        let Some((values, _)) = self.db.get_session_todos(session_id).await? else {
            return Ok(None);
        };
        let todos: Vec<TodoItem> = values
            .into_iter()
            .filter_map(|v| serde_json::from_value(v).ok())
            .collect();
        Ok(Some(todos))
    }

    async fn save(&self, session_id: &str, todos: &[TodoItem]) -> anyhow::Result<()> {
        let values: Vec<_> = todos
            .iter()
            .filter_map(|t| serde_json::to_value(t).ok())
            .collect();
        self.db.upsert_session_todos(session_id, &values).await?;
        self.emit_updated(session_id, todos).await;
        Ok(())
    }

    async fn clear(&self, session_id: &str) -> anyhow::Result<()> {
        self.db.delete_session_todos(session_id).await?;
        self.emit_updated(session_id, &[]).await;
        Ok(())
    }
}
