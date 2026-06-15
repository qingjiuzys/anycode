//! Inject active hierarchical plan tree into LLM context each turn.

use super::AgentRuntime;
use anycode_core::{
    format_plan_tree_summary, plan_tree_is_empty, Message, MessageContent, MessageRole,
    ANYCODE_CONTEXT_USER_METADATA_KEY,
};
use std::collections::HashMap;
use uuid::Uuid;

pub(crate) const PLAN_TREE_CONTEXT_PREFIX: &str = anycode_core::PLAN_TREE_CONTEXT_PREFIX;

pub(crate) fn is_plan_tree_context_message(msg: &Message) -> bool {
    matches!(&msg.content, MessageContent::Text(t) if t.trim_start().starts_with(PLAN_TREE_CONTEXT_PREFIX))
}

impl AgentRuntime {
    pub(super) fn sync_plan_tree_context(&self, messages: &mut Vec<Message>) {
        messages.retain(|m| !is_plan_tree_context_message(m));
        let Some(services) = self.tool_services.lock().ok().and_then(|g| g.clone()) else {
            return;
        };
        let tree = services.plan_tree();
        if plan_tree_is_empty(&tree) {
            return;
        }
        let summary = format_plan_tree_summary(&tree);
        if summary.trim().is_empty() {
            return;
        }
        let mut metadata = HashMap::new();
        metadata.insert(
            ANYCODE_CONTEXT_USER_METADATA_KEY.to_string(),
            serde_json::Value::Bool(true),
        );
        messages.push(Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(summary),
            timestamp: chrono::Utc::now(),
            metadata,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn detects_plan_tree_context_message() {
        let msg = Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text(format!("{PLAN_TREE_CONTEXT_PREFIX}\n\n[x] Root (root)")),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        assert!(is_plan_tree_context_message(&msg));
    }
}
