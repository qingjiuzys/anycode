use crate::limits::file_read_max_bytes;
use crate::paths::resolve_path_fields;
use anycode_core::prelude::*;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Deserialize;
use std::time::Instant;

pub struct FileReadTool {
    pub sandbox_mode: bool,
}

impl FileReadTool {
    pub fn new(sandbox_mode: bool) -> Self {
        Self { sandbox_mode }
    }
}

#[derive(Deserialize)]
struct ReadInput {
    file_path: String,
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "FileRead"
    }

    fn description(&self) -> &str {
        "Read file contents. Text is returned as UTF-8; binary returns metadata + base64 preview. Large files are rejected before full read (see ANYCODE_FILE_READ_MAX_BYTES)."
    }

    fn api_tool_description(&self) -> String {
        format!(
            "{}\n\n\
            Read a single file from disk for analysis or before edits.\n\
            - UTF-8 text is returned as a string; detected binary may return base64 preview + metadata.\n\
            - A maximum byte budget applies (env ANYCODE_FILE_READ_MAX_BYTES); read smaller ranges or use shell tools for huge artifacts.\n\
            - Always use absolute or sandbox-relative paths consistent with the task working directory.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Path to the file to read (absolute or under sandbox cwd)"
                }
            },
            "required": ["file_path"]
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
        let wd = input.working_directory.as_deref();
        let sandbox_in = input.sandbox_mode;
        let ReadInput { file_path } =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        let path = resolve_path_fields(self.sandbox_mode, sandbox_in, wd, &file_path)?;
        let max = file_read_max_bytes();

        let meta = tokio::fs::metadata(&path).await;
        let meta = match meta {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(tool_fail(
                    start,
                    serde_json::json!({
                        "error": "File not found",
                        "path": path.to_string_lossy()
                    }),
                    "File not found",
                ));
            }
            Err(e) => return Err(CoreError::IoError(e)),
        };

        if !meta.is_file() {
            return Ok(tool_fail(
                start,
                serde_json::json!({
                    "error": "Not a regular file",
                    "path": path.to_string_lossy()
                }),
                "Not a file",
            ));
        }

        let len = meta.len();
        if len > max {
            return Ok(tool_fail(
                start,
                serde_json::json!({
                    "error": "File too large",
                    "path": path.to_string_lossy(),
                    "size_bytes": len,
                    "max_bytes": max,
                    "hint": "Increase ANYCODE_FILE_READ_MAX_BYTES or read a smaller range"
                }),
                "File too large",
            ));
        }

        let bytes = tokio::fs::read(&path).await?;
        let path_s = path.to_string_lossy().to_string();

        let result = match std::str::from_utf8(&bytes) {
            Ok(text) => serde_json::json!({
                "content": text,
                "path": path_s,
                "size_bytes": bytes.len(),
                "encoding": "utf-8"
            }),
            Err(_) => {
                let prev = bytes.len().min(384);
                serde_json::json!({
                    "path": path_s,
                    "size_bytes": bytes.len(),
                    "encoding": "binary",
                    "preview_base64": B64.encode(&bytes[..prev]),
                    "note": "Non-UTF-8 file; use specialized tools for images/PDFs"
                })
            }
        };

        Ok(ToolOutput {
            result,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn tool_fail(start: Instant, result: serde_json::Value, err: &str) -> ToolOutput {
    ToolOutput {
        result,
        error: Some(err.to_string()),
        duration_ms: start.elapsed().as_millis() as u64,
    }
}
