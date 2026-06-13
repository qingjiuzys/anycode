//! Search indexed project knowledge paths (RAG-lite).

use crate::services::ToolServices;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

#[derive(Deserialize)]
struct KnowledgeIn {
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

fn default_limit() -> usize {
    8
}

pub struct KnowledgeSearchTool {
    #[allow(dead_code)] // registry DI slot; search uses cwd + knowledge_index directly
    services: Arc<ToolServices>,
}

impl KnowledgeSearchTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self { services }
    }
}

#[async_trait]
impl Tool for KnowledgeSearchTool {
    fn name(&self) -> &str {
        "KnowledgeSearch"
    }
    fn description(&self) -> &str {
        "Search the current project's indexed knowledge base (configured paths in Digital Workbench). \
         Returns snippets from docs/, references/, and other indexed folders."
    }
    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 20 }
            },
            "required": ["query"]
        })
    }
    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }
    fn security_policy(&self) -> Option<&SecurityPolicy> {
        None
    }
    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let k: KnowledgeIn =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;
        let cwd = input
            .working_directory
            .as_deref()
            .map(Path::new)
            .unwrap_or_else(|| Path::new("."));
        match crate::knowledge_index::search_chunks_hybrid(cwd, &k.query, k.limit).await {
            Ok(hits) if hits.is_empty() => Ok(ToolOutput {
                result: json!({ "hits": [], "hint": "no matches; configure knowledge paths in dashboard and reindex" }),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }),
            Ok(hits) => Ok(ToolOutput {
                result: json!({ "hits": hits }),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            }),
            Err(e) => Ok(ToolOutput {
                result: json!({ "error": e.to_string() }),
                error: Some(e.to_string()),
                duration_ms: start.elapsed().as_millis() as u64,
            }),
        }
    }
}
