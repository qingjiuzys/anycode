//! 工具结果截断与产物提取（纯函数，便于单测）。

use anycode_core::prelude::*;
use anycode_core::Artifact;
use std::collections::HashMap;

pub(crate) fn truncate_text(s: String, max_bytes: usize) -> (String, bool) {
    if s.len() <= max_bytes {
        return (s, false);
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = s;
    out.truncate(end);
    out.push_str("\n...<truncated>");
    (out, true)
}

pub(crate) fn extract_artifacts(tool_call: &ToolCall, tool_output: &ToolOutput) -> Vec<Artifact> {
    let mut out: Vec<Artifact> = vec![];
    match tool_call.name.as_str() {
        "FileWrite" | "Edit" => {
            if let Some(path) = tool_output
                .result
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            {
                out.push(Artifact {
                    name: "file".to_string(),
                    path: Some(path),
                    content: None,
                    metadata: HashMap::new(),
                });
            }
        }
        "NotebookEdit" => {
            if let Some(path) = tool_output
                .result
                .get("notebook_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            {
                out.push(Artifact {
                    name: "notebook".to_string(),
                    path: Some(path),
                    content: None,
                    metadata: HashMap::new(),
                });
            }
        }
        "Bash" => {
            let command = tool_call
                .input
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let stdout = tool_output
                .result
                .get("stdout")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let stderr = tool_output
                .result
                .get("stderr")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let exit_code = tool_output
                .result
                .get("exit_code")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            let mut metadata = HashMap::new();
            metadata.insert("command".to_string(), serde_json::Value::String(command));
            metadata.insert("exit_code".to_string(), exit_code);

            let mut combined = String::new();
            if !stdout.is_empty() {
                combined.push_str("== stdout ==\n");
                combined.push_str(&stdout);
                if !stdout.ends_with('\n') {
                    combined.push('\n');
                }
            }
            if !stderr.is_empty() {
                combined.push_str("== stderr ==\n");
                combined.push_str(&stderr);
                if !stderr.ends_with('\n') {
                    combined.push('\n');
                }
            }

            let (content, _truncated) = truncate_text(combined, 4 * 1024);

            out.push(Artifact {
                name: "bash".to_string(),
                path: None,
                content: if content.trim().is_empty() {
                    None
                } else {
                    Some(content)
                },
                metadata,
            });
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn truncate_empty_untouched() {
        let (s, t) = truncate_text(String::new(), 10);
        assert!(!t);
        assert!(s.is_empty());
    }

    #[test]
    fn truncate_ascii_adds_marker() {
        let (s, t) = truncate_text("abcdefgh".to_string(), 4);
        assert!(t);
        assert!(s.contains("<truncated>"));
        assert_eq!(s.chars().count() < 20, true);
    }

    #[test]
    fn extract_bash_artifact_has_command_metadata() {
        let tc = ToolCall {
            id: "1".into(),
            name: "Bash".into(),
            input: json!({ "command": "echo hi" }),
        };
        let out = ToolOutput {
            result: json!({ "stdout": "hi\n", "stderr": "", "exit_code": 0 }),
            error: None,
            duration_ms: 1,
        };
        let arts = extract_artifacts(&tc, &out);
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].name, "bash");
        assert!(arts[0].metadata.get("command").is_some());
    }
}
