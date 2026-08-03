//! `TodoWrite` — session-scoped todo list (aligned with Claude Code todos semantics).

use crate::services::{TodoItem, ToolServices};
use crate::session_store::resolve_session_key;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

pub struct TodoWriteTool {
    services: Arc<ToolServices>,
}

impl TodoWriteTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[derive(Deserialize)]
struct TodoIn {
    #[serde(default)]
    id: Option<String>,
    content: String,
    status: String,
    #[serde(default, alias = "active_form")]
    active_form: Option<String>,
}

#[derive(Deserialize)]
struct TwInput {
    todos: Vec<TodoIn>,
}

#[async_trait]
impl Tool for TodoWriteTool {
    fn name(&self) -> &str {
        "TodoWrite"
    }

    fn description(&self) -> &str {
        "Update the session todo checklist (scoped to the current dashboard session)."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "todos": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "content": { "type": "string" },
                            "status": { "type": "string", "enum": ["pending", "in_progress", "completed"] },
                            "activeForm": { "type": "string", "description": "Present continuous form shown in spinner when in_progress (e.g. \"Running tests\")" }
                        },
                        "required": ["content", "status"]
                    }
                }
            },
            "required": ["todos"]
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Auto
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let session_id = input.dashboard_session_id.clone();
        let session_key = resolve_session_key(session_id.as_deref());
        self.services.hydrate_todos(session_id.as_deref()).await;
        let tw: TwInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let mapped: Vec<TodoItem> = tw
            .todos
            .into_iter()
            .map(|t| TodoItem {
                id: t.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
                content: t.content,
                status: t.status,
                active_form: t.active_form,
            })
            .collect();
        let (old, new) = self.services.replace_todos(session_id.as_deref(), mapped);
        self.services
            .persist_todos(session_id.as_deref(), &new)
            .await;
        Ok(ToolOutput {
            result: serde_json::json!({
                "oldTodos": old,
                "newTodos": new,
                "sessionId": session_key,
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
