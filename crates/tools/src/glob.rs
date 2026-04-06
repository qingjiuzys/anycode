use crate::limits::GLOB_MAX_FILES;
use crate::paths::resolve_path_fields;
use anycode_core::prelude::*;
use async_trait::async_trait;
use globset::GlobBuilder;
use serde::Deserialize;
use std::path::Path;
use std::time::Instant;

pub struct GlobTool {
    pub sandbox_mode: bool,
}

impl GlobTool {
    pub fn new(sandbox_mode: bool) -> Self {
        Self { sandbox_mode }
    }
}

#[derive(Deserialize)]
struct GlobInput {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
}

fn rel_as_unix(rel: &Path) -> String {
    rel.iter()
        .map(|c| c.to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> &str {
        "Find files matching a glob pattern (supports ** / * / ?). Output includes durationMs, numFiles, filenames, truncated (cap 100)."
    }

    fn api_tool_description(&self) -> String {
        format!(
            "{}\n\n\
            File discovery by glob from an optional root (`path`).\n\
            - Use `**` for recursive patterns (e.g. `src/**/*.rs`).\n\
            - Results are capped (truncated=true when over limit); narrow the pattern if needed.\n\
            - Paths respect sandbox_mode relative to the task working directory.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string" },
                "path": { "type": "string", "description": "Search root directory" }
            },
            "required": ["pattern"]
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
        let g: GlobInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        let path_arg = g.path.unwrap_or_else(|| ".".to_string());
        let root = resolve_path_fields(self.sandbox_mode, sandbox_in, wd, &path_arg)?;

        if !root.exists() {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": "Search root does not exist",
                    "path": root.to_string_lossy()
                }),
                error: Some("not found".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let matcher = match GlobBuilder::new(&g.pattern).build() {
            Ok(g) => g.compile_matcher(),
            Err(e) => {
                return Ok(ToolOutput {
                    result: serde_json::json!({ "error": format!("invalid glob: {}", e) }),
                    error: Some("invalid glob".into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        };

        let root_abs = root.canonicalize().unwrap_or(root);
        let mut filenames = Vec::new();
        let mut truncated = false;

        for entry in walkdir::WalkDir::new(&root_abs)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let rel = path.strip_prefix(&root_abs).unwrap_or(path);
            let rel_unix = rel_as_unix(rel);
            if matcher.is_match(Path::new(&rel_unix)) {
                if filenames.len() >= GLOB_MAX_FILES {
                    truncated = true;
                    break;
                }
                filenames.push(path.to_string_lossy().to_string());
            }
        }

        let duration_ms = start.elapsed().as_millis() as u64;
        Ok(ToolOutput {
            result: serde_json::json!({
                "durationMs": duration_ms,
                "numFiles": filenames.len(),
                "filenames": filenames,
                "truncated": truncated,
                "matches": filenames,
                "count": filenames.len()
            }),
            error: None,
            duration_ms,
        })
    }
}
