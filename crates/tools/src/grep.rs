use crate::limits::GREP_MAX_JSON_LINES;
use crate::paths::resolve_read_path_fields;
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
    #[serde(default)]
    glob: Option<String>,
    /// "content" | "files_with_matches" | "count"
    #[serde(default)]
    output_mode: Option<String>,
    #[serde(rename = "-B", default)]
    before: Option<u32>,
    #[serde(rename = "-A", default)]
    after: Option<u32>,
    #[serde(rename = "-C", default)]
    context_c: Option<u32>,
    #[serde(default)]
    context: Option<u32>,
    #[serde(rename = "-n", default)]
    #[allow(dead_code)] // 对齐 Claude Code schema；JSON 输出恒含行号，等价于默认 true
    line_number: Option<bool>,
    #[serde(rename = "-i", default)]
    case_insensitive: Option<bool>,
    #[serde(rename = "-o", default)]
    only_matching: Option<bool>,
    #[serde(rename = "type", default)]
    file_type: Option<String>,
    #[serde(default)]
    head_limit: Option<usize>,
    #[serde(default)]
    offset: Option<usize>,
    #[serde(default)]
    multiline: Option<bool>,
}

impl GrepInput {
    fn context_lines(&self) -> Option<u32> {
        self.context.or(self.context_c)
    }
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
            - `output_mode`: \"content\" (default, structured matches with context), \"files_with_matches\" (file paths only), \"count\" (per-file match counts).\n\
            - Context flags `-B`/`-A`/`-C`/`context`, `-n`, `-i`, `-o`, `type`, `head_limit`, `offset` and `multiline` mirror Claude Code's Grep tool.\n\
            - Output may be truncated when too many matches; refine pattern or path.\n\
            - Prefer Grep for exact symbol/string search; use Glob for filename patterns.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "pattern": { "type": "string", "description": "The regular expression pattern to search for in file contents (ripgrep regex)" },
                "path": { "type": "string", "description": "File or directory to search in (rg PATH). Defaults to current working directory." },
                "glob": { "type": "string", "description": "Glob pattern to filter files (e.g. \"*.js\", \"*.{ts,tsx}\") - maps to rg --glob" },
                "output_mode": {
                    "type": "string",
                    "enum": ["content", "files_with_matches", "count"],
                    "description": "Output mode: \"content\" shows matching lines (supports -A/-B/-C context, -n line numbers, head_limit), \"files_with_matches\" shows file paths (supports head_limit), \"count\" shows match counts (supports head_limit). Defaults to \"content\"."
                },
                "-B": { "type": "integer", "description": "Number of lines to show before each match (rg -B). Requires output_mode: \"content\", ignored otherwise." },
                "-A": { "type": "integer", "description": "Number of lines to show after each match (rg -A). Requires output_mode: \"content\", ignored otherwise." },
                "-C": { "type": "integer", "description": "Alias for context." },
                "context": { "type": "integer", "description": "Number of lines to show before and after each match (rg -C). Requires output_mode: \"content\", ignored otherwise." },
                "-n": { "type": "boolean", "description": "Show line numbers in output (rg -n). Requires output_mode: \"content\", ignored otherwise. Defaults to true." },
                "-i": { "type": "boolean", "description": "Case insensitive search (rg -i)" },
                "-o": { "type": "boolean", "description": "Print only the matched (non-empty) parts of each matching line, one match per output line (rg -o / --only-matching). Requires output_mode: \"content\", ignored otherwise. Defaults to false." },
                "type": { "type": "string", "description": "File type to search (rg --type). Common types: js, py, rust, go, java, etc." },
                "head_limit": { "type": "integer", "description": "Limit output to first N lines/entries. Pass 0 for unlimited (use sparingly — large result sets waste context)." },
                "offset": { "type": "integer", "description": "Skip first N lines/entries before applying head_limit. Defaults to 0." },
                "multiline": { "type": "boolean", "description": "Enable multiline mode where . matches newlines and patterns can span lines (rg -U --multiline-dotall). Default: false." }
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

        let path_arg = g.path.clone().unwrap_or_else(|| ".".to_string());
        let root = resolve_read_path_fields(self.sandbox_mode, sandbox_in, wd, &path_arg)?;
        let pat = g.pattern.clone();
        let root_m = root.clone();
        let mode = g
            .output_mode
            .clone()
            .unwrap_or_else(|| "content".to_string());
        let is_files = mode == "files_with_matches";
        let is_count = mode == "count";
        let use_json = !is_files && !is_count;

        let mut args: Vec<String> = Vec::new();
        if use_json {
            args.push("--json".into());
        } else if is_files {
            args.push("--files-with-matches".into());
        } else {
            args.push("--count".into());
        }
        args.push("--hidden".into());
        args.push("--glob".into());
        args.push("!.git/*".into());
        if let Some(gp) = &g.glob {
            args.push("--glob".into());
            args.push(gp.clone());
        }
        if let Some(n) = g.before {
            args.push("-B".into());
            args.push(n.to_string());
        }
        if let Some(n) = g.after {
            args.push("-A".into());
            args.push(n.to_string());
        }
        if let Some(n) = g.context_lines() {
            args.push("-C".into());
            args.push(n.to_string());
        }
        if g.case_insensitive.unwrap_or(false) {
            args.push("-i".into());
        }
        if g.only_matching.unwrap_or(false) {
            args.push("-o".into());
        }
        if let Some(t) = &g.file_type {
            args.push("--type".into());
            args.push(t.clone());
        }
        if g.multiline.unwrap_or(false) {
            args.push("-U".into());
            args.push("--multiline-dotall".into());
        }
        args.push(pat.clone());
        args.push(root_m.to_string_lossy().into_owned());

        let (stdout, stderr, code, rg_ok) = tokio::task::spawn_blocking(move || {
            let out = Command::new("rg").args(&args).output();
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

        let offset = g.offset.unwrap_or(0);
        let head_limit = g.head_limit;
        let apply_paging = |lines: Vec<String>| -> (Vec<String>, bool) {
            let mut out: Vec<String> = Vec::new();
            let mut truncated = false;
            for (idx, line) in lines.into_iter().enumerate() {
                if idx < offset {
                    continue;
                }
                if let Some(limit) = head_limit {
                    if limit == 0 {
                        // unlimited (still bounded by GREP_MAX_JSON_LINES below for content mode)
                    } else if out.len() >= limit {
                        truncated = true;
                        break;
                    }
                }
                out.push(line);
            }
            (out, truncated)
        };

        if !use_json {
            let raw_lines: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();
            let (out_lines, truncated) = apply_paging(raw_lines);
            let result = if is_count {
                serde_json::json!({
                    "counts": out_lines,
                    "exit_code": code,
                    "truncated": truncated,
                    "stderr": stderr
                })
            } else {
                serde_json::json!({
                    "files": out_lines,
                    "file_count": out_lines.len(),
                    "exit_code": code,
                    "truncated": truncated,
                    "stderr": stderr
                })
            };
            return Ok(ToolOutput {
                result,
                error: None,
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
        if offset > 0 || head_limit.is_some() {
            let mut kept: Vec<Value> = Vec::new();
            for (idx, v) in structured.into_iter().enumerate() {
                if idx < offset {
                    continue;
                }
                if let Some(limit) = head_limit {
                    if limit > 0 && kept.len() >= limit {
                        truncated = true;
                        break;
                    }
                }
                kept.push(v);
            }
            structured = kept;
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
