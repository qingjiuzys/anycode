use crate::limits::GREP_MAX_JSON_LINES;
use crate::paths::resolve_path_fields;
use anycode_core::prelude::*;
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::Value;
use std::process::Command;
use std::time::Instant;

pub struct GrepTool {
    pub sandbox_mode: bool,
}

impl GrepTool {
    pub fn new(sandbox_mode: bool) -> Self {
        Self { sandbox_mode }
    }
}

#[derive(Deserialize)]
struct GrepInput {
    pattern: String,
    #[serde(default)]
    path: Option<String>,
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> &str {
        "Search with ripgrep (--json). Parses match lines; caps output volume."
    }

    fn api_tool_description(&self) -> String {
        format!(
            "{}\n\n\
            Codebase search via ripgrep with `--json` for structured matches.\n\
            - `pattern` uses ripgrep regex syntax.\n\
            - Optional `path` scopes the search root; defaults to workspace / cwd under sandbox rules.\n\
            - Output may be truncated when too many matches; refine pattern or path.\n\
            - Prefer Grep for exact symbol/string search; use Glob for filename patterns.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "Rust regex (ripgrep)" },
                "path": { "type": "string", "description": "Directory root" }
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
        let g: GrepInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        let path_arg = g.path.unwrap_or_else(|| ".".to_string());
        let root = resolve_path_fields(self.sandbox_mode, sandbox_in, wd, &path_arg)?;
        let pat = g.pattern.clone();
        let root_m = root.clone();

        let (stdout, stderr, code, rg_ok) = tokio::task::spawn_blocking(move || {
            let out = Command::new("rg")
                .arg("--json")
                .arg("--hidden")
                .arg("--glob")
                .arg("!.git/*")
                .arg(&pat)
                .arg(&root_m)
                .output();
            match out {
                Ok(o) => {
                    let code = o.status.code();
                    let ok = o.status.success() || code == Some(1);
                    (
                        String::from_utf8_lossy(&o.stdout).to_string(),
                        String::from_utf8_lossy(&o.stderr).to_string(),
                        code,
                        ok,
                    )
                }
                Err(e) => (String::new(), e.to_string(), None, false),
            }
        })
        .await
        .map_err(|e| CoreError::Other(anyhow::anyhow!("rg join: {}", e)))?;

        if !rg_ok {
            return Ok(ToolOutput {
                result: serde_json::json!({
                    "error": "ripgrep not available or failed",
                    "stderr": stderr,
                    "exit_code": code
                }),
                error: Some("rg failed".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let mut structured: Vec<Value> = Vec::new();
        let mut raw_lines: Vec<String> = Vec::new();
        let mut truncated = false;
        for line in stdout.lines() {
            if structured.len() + raw_lines.len() >= GREP_MAX_JSON_LINES {
                truncated = true;
                break;
            }
            if let Ok(v) = serde_json::from_str::<Value>(line) {
                if v.get("type").and_then(|t| t.as_str()) == Some("match") {
                    structured.push(v);
                } else {
                    raw_lines.push(line.to_string());
                }
            } else {
                raw_lines.push(line.to_string());
            }
        }
        let duration_ms = start.elapsed().as_millis() as u64;

        Ok(ToolOutput {
            result: serde_json::json!({
                "matches": structured,
                "raw_lines": raw_lines,
                "match_count": structured.len(),
                "exit_code": code,
                "truncated": truncated,
                "stderr": stderr
            }),
            error: None,
            duration_ms,
        })
    }
}
