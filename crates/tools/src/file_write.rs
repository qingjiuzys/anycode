use crate::paths::resolve_path_fields;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use std::time::Instant;

pub struct FileWriteTool {
    security_policy: SecurityPolicy,
}

impl FileWriteTool {
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

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str {
        "FileWrite"
    }

    fn description(&self) -> &str {
        "Create or overwrite a file with new content."
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "file_path": {
                    "type": "string",
                    "description": "Absolute path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                }
            },
            "required": ["file_path", "content"]
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

        #[derive(Deserialize)]
        struct WriteInput {
            file_path: String,
            content: String,
        }

        let wd = input.working_directory.as_deref();
        let sandbox_in = input.sandbox_mode;
        let write_input: WriteInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        let target = resolve_path_fields(
            self.security_policy.sandbox_mode,
            sandbox_in,
            wd,
            &write_input.file_path,
        )?;

        if let Some(parent) = target.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::write(&target, &write_input.content).await?;

        let result = serde_json::json!({
            "success": true,
            "path": target.to_string_lossy()
        });

        Ok(ToolOutput {
            result,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
