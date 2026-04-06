//! 会话消息模型。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// 消息角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// 消息内容
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    ToolUse {
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        is_error: bool,
    },
}

/// 消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub role: MessageRole,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, serde_json::Value>,
}
