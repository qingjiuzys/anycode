use crate::limits::file_read_max_bytes;
use crate::paths::resolve_read_path_fields;
use anycode_core::prelude::*;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Deserialize;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, BufReader};

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
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    pages: Option<String>,
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
            - `offset`/`limit` select a line range for large files (offset is 1-based, mirroring Claude Code).\n\
            - `pages` selects a page range for PDF files (e.g. \"1-5\", \"10-20\", max 20 pages).\n\
            - A maximum byte budget applies for full reads (env ANYCODE_FILE_READ_MAX_BYTES); line-range reads bypass it.\n\
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
                },
                "offset": {
                    "type": "integer",
                    "description": "The line number to start reading from (1-based). Only provide if the file is too large to read at once."
                },
                "limit": {
                    "type": "integer",
                    "description": "The number of lines to read. Only provide if the file is too large to read at once."
                },
                "pages": {
                    "type": "string",
                    "description": "Page range for PDF files (e.g., \"1-5\", \"3\", \"10-20\"). Only applicable to PDF files. Maximum 20 pages per request."
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
        let ReadInput {
            file_path,
            offset,
            limit,
            pages,
        } = serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        let path = resolve_read_path_fields(self.sandbox_mode, sandbox_in, wd, &file_path)?;
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

        let path_s = path.to_string_lossy().to_string();
        let len = meta.len();

        // PDF page-range reads: signal unsupported parsing but keep schema-compatible.
        if pages.is_some() {
            return Ok(tool_fail(
                start,
                serde_json::json!({
                    "error": "PDF page extraction not available in FileRead",
                    "path": path_s,
                    "size_bytes": len,
                    "hint": "Use the pdf skill / dedicated PDF tools for page-range extraction"
                }),
                "PDF pages unsupported in FileRead",
            ));
        }

        // Line-range reads (offset/limit) stream only the requested window.
        if offset.is_some() || limit.is_some() {
            let offset = offset.unwrap_or(1).max(1);
            let limit = limit.unwrap_or(usize::MAX);
            let file = tokio::fs::File::open(&path).await?;
            let mut reader = BufReader::new(file);
            let mut buf = String::new();
            let mut lines: Vec<String> = Vec::new();
            let mut line_no = 0usize;
            loop {
                buf.clear();
                let n = reader.read_line(&mut buf).await?;
                if n == 0 {
                    break;
                }
                line_no += 1;
                if line_no < offset {
                    continue;
                }
                if lines.len() >= limit {
                    break;
                }
                // strip trailing newline for consistent output
                let trimmed = buf.trim_end_matches(['\n', '\r']);
                lines.push(trimmed.to_string());
            }
            let total = lines.len();
            let truncated = offset + total.saturating_sub(1) < line_no;
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "content": lines.join("\n"),
                    "path": path_s,
                    "size_bytes": len,
                    "encoding": "utf-8",
                    "offset": offset,
                    "limit": total,
                    "end_line": offset + total.saturating_sub(1),
                    "total_lines_seen": line_no,
                    "truncated": truncated
                }),
                error: None,
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        if len > max {
            return Ok(tool_fail(
                start,
                serde_json::json!({
                    "error": "File too large",
                    "path": path_s,
                    "size_bytes": len,
                    "max_bytes": max,
                    "hint": "Increase ANYCODE_FILE_READ_MAX_BYTES or read a smaller range with offset/limit"
                }),
                "File too large",
            ));
        }

        let bytes = tokio::fs::read(&path).await?;

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
