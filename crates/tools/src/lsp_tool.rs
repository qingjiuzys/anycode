//! `LSP` — 语言服务占位（子进程/stdio 集成可后续接入）。

use anycode_core::prelude::*;
use async_trait::async_trait;
use std::time::Instant;

pub struct LspTool {
    security_policy: SecurityPolicy,
}

impl LspTool {
    pub fn new() -> Self {
        Self {
            security_policy: SecurityPolicy::sensitive_mutation(),
        }
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "LSP"
    }

    fn description(&self) -> &str {
        "Language Server Protocol queries (symbols, diagnostics, etc.). Not wired in anyCode yet."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "additionalProperties": true,
            "description": "LSP method and params"
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.security_policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        #[cfg(feature = "tools-lsp")]
        {
            if let Ok(cmd) = std::env::var("ANYCODE_LSP_COMMAND") {
                let cmd = cmd.trim();
                if !cmd.is_empty() {
                    return crate::lsp_stdio::lsp_forward_shell(&input.input, cmd).await;
                }
            }
        }

        let start = Instant::now();
        Ok(ToolOutput {
            result: serde_json::json!({
                "status": "unsupported",
                "error": "LSP client not configured",
                "hint": "Build with --features tools-lsp and set ANYCODE_LSP_COMMAND to an LSP bridge command.",
                "echo": input.input
            }),
            error: Some("lsp stub".into()),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
