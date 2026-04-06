//! `TodoWrite` — 会话内 todo 列表（内存，对齐 `todos` 数组语义）。

use crate::services::{TodoItem, ToolServices};
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

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
    id: String,
    content: String,
    status: String,
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
        "Update the session todo checklist (in-memory for this process)."
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
                            "id": { "type": "string" },
                            "content": { "type": "string" },
                            "status": { "type": "string" }
                        },
                        "required": ["id", "content", "status"]
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
        let tw: TwInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let mapped: Vec<TodoItem> = tw
            .todos
            .into_iter()
            .map(|t| TodoItem {
                id: t.id,
                content: t.content,
                status: t.status,
            })
            .collect();
        let (old, new) = self.services.replace_todos(mapped);
        Ok(ToolOutput {
            result: serde_json::json!({
                "oldTodos": old,
                "newTodos": new
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
