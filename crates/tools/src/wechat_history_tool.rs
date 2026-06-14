//! Read-only local WeChat chat history query tool for agents.

use crate::services::ToolServices;
use anycode_core::prelude::*;
use anycode_wechat_history::{query_history, WechatHistoryOutputFormat, WechatHistoryQuery};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use std::time::Instant;

pub struct QueryWeChatHistoryTool {
    services: Arc<ToolServices>,
    policy: SecurityPolicy,
}

impl QueryWeChatHistoryTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            services,
            policy: SecurityPolicy::sensitive_mutation(),
        }
    }

    fn error_hint(message: &str) -> Option<&'static str> {
        let m = message.to_ascii_lowercase();
        if m.contains("keymap")
            || m.contains("key map")
            || m.contains("wechat_keys")
            || m.contains("keymappath")
        {
            Some(
                "Run: anycode wechat history setup (WeChat logged in; sudo + temporary SIP disable may be required)",
            )
        } else if m.contains("sqlcipher") {
            Some("Run: brew install sqlcipher, then anycode wechat history setup")
        } else if m.contains("datadir") || m.contains("data dir") || m.contains("db_storage") {
            Some("Run: anycode wechat history setup to detect local WeChat db_storage")
        } else {
            None
        }
    }
}

#[derive(Debug, Deserialize)]
struct QueryWeChatHistoryIn {
    date: String,
    conversation: Option<String>,
    keyword: Option<String>,
    timezone: Option<String>,
    limit: Option<usize>,
    #[serde(default)]
    include_group_sender: bool,
    #[serde(default = "default_true")]
    include_attachments: bool,
    #[serde(default)]
    parse_files: bool,
    #[serde(default)]
    attachment_types: Option<Vec<String>>,
    #[serde(default)]
    max_file_parse_bytes: Option<u64>,
    #[serde(default)]
    format: Option<String>,
}

fn default_true() -> bool {
    true
}

#[async_trait]
impl Tool for QueryWeChatHistoryTool {
    fn name(&self) -> &str {
        "QueryWeChatHistory"
    }

    fn description(&self) -> &str {
        "Query local WeChat chat history for a calendar day (read-only). Default backend \
         is `sqlcipher_key_map`: read via `sqlcipher` CLI + key map JSON on a temporary DB snapshot \
         (never opens live WeChat DB files) — no HTTP \
         port (`brew install sqlcipher`). Optional backends: plain SQLite export or legacy \
         chatlog HTTP. Before first use, run the `wechat-daily-history` skill \
         or `anycode wechat history setup` to detect db_storage, extract/write keys, and merge \
         config. Returns structured records, attachment metadata, optional Excel previews, \
         and/or a Markdown table."
    }

    fn schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "required": ["date"],
            "properties": {
                "date": {
                    "type": "string",
                    "description": "Local calendar date in YYYY-MM-DD"
                },
                "conversation": {
                    "type": "string",
                    "description": "Optional conversation filter (wxid, group id, nickname, or remark)"
                },
                "keyword": {
                    "type": "string",
                    "description": "Optional keyword filter on message content or sender"
                },
                "timezone": {
                    "type": "string",
                    "description": "IANA timezone for day boundaries (default from config, usually Asia/Shanghai)"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max rows to return (capped by config maxRowsPerQuery)"
                },
                "include_group_sender": {
                    "type": "boolean",
                    "description": "Include group sender column in markdown table output"
                },
                "include_attachments": {
                    "type": "boolean",
                    "description": "Include file/app attachment metadata (default true)"
                },
                "parse_files": {
                    "type": "boolean",
                    "description": "Parse local files (Excel/text) when attachment path resolves (default false)"
                },
                "attachment_types": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional filter: file, image, voice, video, link, mini_program, all"
                },
                "max_file_parse_bytes": {
                    "type": "integer",
                    "description": "Max bytes for local file parsing (default 20MB)"
                },
                "format": {
                    "type": "string",
                    "enum": ["records", "markdown_table"],
                    "description": "Output shape: structured records (default) or markdown_table"
                }
            }
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let m: QueryWeChatHistoryIn = serde_json::from_value(input.input).map_err(|e| {
            CoreError::ConfigError(format!("invalid QueryWeChatHistory input: {e}"))
        })?;

        let config = self.services.wechat_history_config();
        if !config.enabled {
            return Ok(ToolOutput {
                result: json!({
                    "ok": false,
                    "error": "wechatHistory.enabled is false; enable it in ~/.anycode/config.json"
                }),
                error: Some("wechat history disabled".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let mut query = WechatHistoryQuery {
            date: m.date,
            conversation: m.conversation,
            keyword: m.keyword,
            timezone: m.timezone.or_else(|| Some(config.default_timezone.clone())),
            limit: m.limit,
            include_group_sender: m.include_group_sender,
            include_attachments: m.include_attachments,
            parse_files: m.parse_files,
            attachment_types: m.attachment_types,
            max_file_parse_bytes: m.max_file_parse_bytes,
        };
        if query
            .timezone
            .as_deref()
            .is_some_and(|s| s.trim().is_empty())
        {
            query.timezone = Some(config.default_timezone.clone());
        }

        let output_format = match m.format.as_deref().map(str::trim).unwrap_or("records") {
            "markdown_table" | "markdown-table" | "table" => {
                WechatHistoryOutputFormat::MarkdownTable
            }
            _ => WechatHistoryOutputFormat::Records,
        };

        match query_history(&config, &query, output_format) {
            Ok(result) => {
                let mut payload = json!({
                    "ok": true,
                    "date": result.date,
                    "timezone": result.timezone,
                    "backend": result.backend,
                    "total": result.total,
                    "truncated": result.truncated,
                    "messages": result.messages,
                });
                if let Some(table) = result.markdown_table {
                    payload["markdown_table"] = json!(table);
                }
                if let Some(stats) = result.attachment_stats {
                    payload["attachment_stats"] = json!(stats);
                }
                if output_format == WechatHistoryOutputFormat::MarkdownTable {
                    payload["format"] = json!("markdown_table");
                } else {
                    payload["format"] = json!("records");
                }
                Ok(ToolOutput {
                    result: payload,
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
            Err(e) => {
                let msg = e.to_string();
                let mut payload = json!({
                    "ok": false,
                    "error": msg,
                });
                if let Some(hint) = Self::error_hint(&msg) {
                    payload["hint"] = json!(hint);
                }
                Ok(ToolOutput {
                    result: payload,
                    error: Some(msg),
                    duration_ms: start.elapsed().as_millis() as u64,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_wechat_history::{WechatHistoryBackendKind, WechatHistoryConfig};

    #[test]
    fn schema_requires_date() {
        let tool = QueryWeChatHistoryTool::new(Arc::new(ToolServices::default()));
        let schema = tool.schema();
        let required = schema
            .get("required")
            .and_then(|v| v.as_array())
            .expect("required array");
        assert!(required.iter().any(|v| v.as_str() == Some("date")));
    }

    #[tokio::test]
    async fn disabled_config_returns_actionable_error() {
        let services = Arc::new(ToolServices::default());
        services.set_wechat_history_config(WechatHistoryConfig {
            enabled: false,
            ..Default::default()
        });
        let tool = QueryWeChatHistoryTool::new(services);
        let out = tool
            .execute(ToolInput {
                name: "QueryWeChatHistory".into(),
                input: json!({ "date": "2026-06-14" }),
                working_directory: None,
                sandbox_mode: false,
            })
            .await
            .unwrap();
        assert_eq!(out.result.get("ok").and_then(|v| v.as_bool()), Some(false));
    }

    #[test]
    fn tool_is_registered_in_catalog_constants() {
        assert_eq!(
            crate::catalog::TOOL_QUERY_WECHAT_HISTORY,
            "QueryWeChatHistory"
        );
        let _ = WechatHistoryBackendKind::SqlcipherKeyMap;
    }
}
