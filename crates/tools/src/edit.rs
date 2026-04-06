//! `Edit` — FileEditTool：唯一匹配 / replace_all、写前 mtime 校验（unexpectedly modified）。

use crate::paths::resolve_path_fields;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::path::Path;
use std::time::Instant;

pub struct EditTool {
    security_policy: SecurityPolicy,
}

impl EditTool {
    pub fn new(sandbox_mode: bool) -> Self {
        Self {
            security_policy: SecurityPolicy {
                allow_commands: vec![],
                deny_commands: vec![],
                require_approval: true,
                sandbox_mode,
                timeout_ms: None,
            },
        }
    }
}

#[derive(Deserialize)]
struct EditInput {
    file_path: String,
    old_string: String,
    new_string: String,
    #[serde(default)]
    replace_all: bool,
}

fn mtime_ns(path: &Path) -> std::io::Result<u128> {
    let meta = std::fs::metadata(path)?;
    let t = meta.modified()?;
    Ok(t.duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos())
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> &str {
        "Replace distinct occurrences in a file. If replace_all is false, old_string must match exactly once. File must not change on disk between read and write."
    }

    fn api_tool_description(&self) -> String {
        format!(
            "{}\n\n\
            String replace in a text file (Claude Code FileEditTool semantics).\n\
            - `replace_all: false` (default): `old_string` must appear **exactly once** in the file; otherwise the tool errors.\n\
            - `replace_all: true`: replace **every** occurrence of `old_string`.\n\
            - `old_string` must be a verbatim slice of file contents (including whitespace and newlines).\n\
            - The implementation checks mtime before write: if the file changed on disk after you read it, the write fails (avoid silent overwrites).\n\
            - Prefer reading the file first when you are unsure of exact context.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": { "type": "string" },
                "old_string": { "type": "string" },
                "new_string": { "type": "string" },
                "replace_all": { "type": "boolean", "description": "Replace all occurrences (default false)" }
            },
            "required": ["file_path", "old_string", "new_string"]
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.security_policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let wd = input.working_directory.as_deref();
        let sandbox_in = input.sandbox_mode;
        let edit: EditInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        if edit.old_string == edit.new_string {
            return Ok(fail(
                &start,
                serde_json::json!({"error": "old_string and new_string must differ"}),
                "invalid input",
            ));
        }

        let path = resolve_path_fields(
            self.security_policy.sandbox_mode,
            sandbox_in,
            wd,
            &edit.file_path,
        )?;

        if !path.exists() {
            return Ok(fail(
                &start,
                serde_json::json!({"error": "File not found"}),
                "File not found",
            ));
        }

        let snap_before = mtime_ns(&path).map_err(CoreError::IoError)?;
        let content = tokio::fs::read_to_string(&path).await?;
        let snap_after_read = mtime_ns(&path).map_err(CoreError::IoError)?;
        if snap_after_read != snap_before {
            return Ok(fail(
                &start,
                serde_json::json!({
                    "error": "File has been unexpectedly modified. Read it again before attempting to write.",
                    "code": "FILE_UNEXPECTEDLY_MODIFIED"
                }),
                "unexpectedly modified",
            ));
        }

        let count = count_non_overlapping(&content, &edit.old_string);
        let new_content = if edit.replace_all {
            if count == 0 {
                return Ok(fail(
                    &start,
                    serde_json::json!({"error": "old_string not found"}),
                    "not found",
                ));
            }
            content.replace(&edit.old_string, &edit.new_string)
        } else {
            match count {
                0 => {
                    return Ok(fail(
                        &start,
                        serde_json::json!({"error": "old_string not found"}),
                        "not found",
                    ));
                }
                1 => content.replacen(&edit.old_string, &edit.new_string, 1),
                _ => {
                    return Ok(fail(
                        &start,
                        serde_json::json!({
                            "error": "old_string is not unique; set replace_all or include more context",
                            "occurrences": count
                        }),
                        "ambiguous",
                    ));
                }
            }
        };

        let snap_pre_write = mtime_ns(&path).map_err(CoreError::IoError)?;
        if snap_pre_write != snap_before {
            return Ok(fail(
                &start,
                serde_json::json!({
                    "error": "File has been unexpectedly modified. Read it again before attempting to write.",
                    "code": "FILE_UNEXPECTEDLY_MODIFIED"
                }),
                "unexpectedly modified",
            ));
        }

        tokio::fs::write(&path, &new_content).await?;

        Ok(ToolOutput {
            result: serde_json::json!({
                "success": true,
                "path": path.to_string_lossy(),
                "replacements": if edit.replace_all { count } else { 1 }
            }),
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

fn count_non_overlapping(haystack: &str, needle: &str) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut n = 0;
    let mut start = 0;
    while let Some(i) = haystack[start..].find(needle) {
        n += 1;
        start += i + needle.len();
    }
    n
}

fn fail(start: &Instant, result: serde_json::Value, err: &str) -> ToolOutput {
    ToolOutput {
        result,
        error: Some(err.to_string()),
        duration_ms: start.elapsed().as_millis() as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::prelude::Tool;

    #[test]
    fn edit_api_description_mentions_replace_all_and_mtime() {
        let t = EditTool::new(false);
        let d = t.api_tool_description();
        assert!(d.contains("replace_all"));
        assert!(d.contains("mtime") || d.contains("changed on disk"));
    }
}
