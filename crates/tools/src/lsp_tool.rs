//! `LSP` — 语言服务（`tools-lsp` 下通过 stdio 子进程转发 JSON-RPC）。

use anycode_core::prelude::*;
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::services::ToolServices;

pub struct LspTool {
    security_policy: SecurityPolicy,
    services: Arc<ToolServices>,
}

impl LspTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            security_policy: SecurityPolicy::sensitive_mutation(),
            services,
        }
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &str {
        "LSP"
    }

    fn description(&self) -> &str {
        "Language Server Protocol queries (symbols, diagnostics, etc.) via stdio when configured (`lsp` in config.json or ANYCODE_LSP_COMMAND)."
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
        let cfg = self.services.lsp_connection_config();
        let cmd = cfg
            .command
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .or_else(|| {
                std::env::var("ANYCODE_LSP_COMMAND").ok().and_then(|s| {
                    let t = s.trim();
                    if t.is_empty() {
                        None
                    } else {
                        Some(t.to_string())
                    }
                })
            });

        #[cfg(feature = "tools-lsp")]
        {
            if let Some(ref c) = cmd {
                return crate::lsp_stdio::lsp_forward_shell(
                    &input.input,
                    c,
                    cfg.workspace_root.as_deref(),
                    cfg.read_timeout,
                )
                .await;
            }
        }

        let start = Instant::now();
        let has_command = cmd.is_some();
        Ok(ToolOutput {
            result: serde_json::json!({
                "status": "unsupported",
                "error": "LSP client not configured",
                "hint": "Build with --features tools-lsp and set `lsp.command` in config.json (with lsp.enabled true) or ANYCODE_LSP_COMMAND.",
                "command_configured": has_command,
                "echo": input.input
            }),
            error: Some("lsp stub".into()),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
