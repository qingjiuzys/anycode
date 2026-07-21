//! 工具结果截断与产物提取（纯函数，便于单测）。

use anycode_core::prelude::*;
use anycode_core::{
    artifact_kind_for_path, artifact_kind_is_inline, artifact_title_for_path, mime_for_path,
    Artifact,
};
use std::collections::HashMap;
use std::path::Path;

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

fn enrich_path_artifact(mut art: Artifact) -> Artifact {
    if let Some(path) = art.path.clone() {
        if art.kind.is_none() {
            art.kind = Some(artifact_kind_for_path(&path).to_string());
        }
        if art.mime.is_none() {
            art.mime = Some(mime_for_path(&path).to_string());
        }
        if art.title.is_none() {
            art.title = Some(artifact_title_for_path(&path));
        }
        if art.bytes.is_none() {
            art.bytes = std::fs::metadata(&path).ok().map(|m| m.len());
        }
        if art.inline.is_none() {
            let kind = art.resolved_kind().to_string();
            art.inline = Some(artifact_kind_is_inline(&kind));
        }
        if art.name.is_empty() || art.name == "file" {
            art.name = art.resolved_kind().to_string();
        }
    }
    art
}

fn parse_artifact_value(v: &serde_json::Value) -> Option<Artifact> {
    if let Some(path) = v.get("path").and_then(|p| p.as_str()) {
        let mut art = Artifact::from_path(path);
        if let Some(kind) = v.get("kind").and_then(|k| k.as_str()) {
            art.kind = Some(kind.to_string());
            art.name = kind.to_string();
        }
        if let Some(mime) = v.get("mime").and_then(|m| m.as_str()) {
            art.mime = Some(mime.to_string());
        }
        if let Some(title) = v.get("title").and_then(|t| t.as_str()) {
            art.title = Some(title.to_string());
        }
        if let Some(bytes) = v.get("bytes").and_then(|b| b.as_u64()) {
            art.bytes = Some(bytes);
        }
        if let Some(preview) = v.get("preview_path").and_then(|p| p.as_str()) {
            art.preview_path = Some(preview.to_string());
        }
        if let Some(inline) = v.get("inline").and_then(|i| i.as_bool()) {
            art.inline = Some(inline);
        }
        return Some(enrich_path_artifact(art));
    }
    None
}

fn artifacts_from_result_json(result: &serde_json::Value) -> Vec<Artifact> {
    let mut out = Vec::new();
    if let Some(arr) = result.get("artifacts").and_then(|a| a.as_array()) {
        for item in arr {
            if let Some(art) = parse_artifact_value(item) {
                out.push(art);
            }
        }
    }
    if let Some(one) = result.get("artifact") {
        if let Some(art) = parse_artifact_value(one) {
            out.push(art);
        }
    }
    out
}

fn artifacts_from_sidecar(path: &str) -> Vec<Artifact> {
    let sidecar = format!("{path}.anycode-artifact.json");
    let Ok(raw) = std::fs::read_to_string(&sidecar) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    if let Some(art) = parse_artifact_value(&v) {
        return vec![art];
    }
    if let Some(arr) = v.as_array() {
        return arr.iter().filter_map(parse_artifact_value).collect();
    }
    Vec::new()
}

fn artifacts_from_stdout_footer(text: &str) -> Vec<Artifact> {
    let mut out = Vec::new();
    for line in text.lines().rev().take(8) {
        let trimmed = line.trim();
        let Some(json_part) = trimmed.strip_prefix("ANYCODE_ARTIFACT:") else {
            continue;
        };
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_part.trim()) {
            if let Some(art) = parse_artifact_value(&v) {
                out.push(art);
                break;
            }
        }
    }
    out
}

fn push_unique(out: &mut Vec<Artifact>, art: Artifact) {
    if let Some(path) = art.path.as_deref() {
        if out.iter().any(|a| a.path.as_deref() == Some(path)) {
            return;
        }
    }
    out.push(art);
}

pub(crate) fn extract_artifacts(tool_call: &ToolCall, tool_output: &ToolOutput) -> Vec<Artifact> {
    let mut out: Vec<Artifact> = vec![];

    for art in artifacts_from_result_json(&tool_output.result) {
        push_unique(&mut out, art);
    }

    // Sidecar next to any path field on the result.
    if let Some(path) = tool_output
        .result
        .get("path")
        .and_then(|v| v.as_str())
        .or_else(|| {
            tool_output
                .result
                .get("notebook_path")
                .and_then(|v| v.as_str())
        })
    {
        for art in artifacts_from_sidecar(path) {
            push_unique(&mut out, art);
        }
    }

    let stdout = tool_output
        .result
        .get("stdout")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    for art in artifacts_from_stdout_footer(stdout) {
        let side_path = art.path.clone();
        push_unique(&mut out, art);
        if let Some(path) = side_path.as_deref() {
            for side in artifacts_from_sidecar(path) {
                push_unique(&mut out, side);
            }
        }
    }

    match tool_call.name.as_str() {
        "FileWrite" | "Edit" => {
            if let Some(path) = tool_output
                .result
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            {
                push_unique(&mut out, enrich_path_artifact(Artifact::from_path(path)));
            }
        }
        "NotebookEdit" => {
            if let Some(path) = tool_output
                .result
                .get("notebook_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            {
                let mut art = Artifact::from_path(path);
                art.kind = Some("notebook".into());
                art.name = "notebook".into();
                art.inline = Some(false);
                push_unique(&mut out, art);
            }
        }
        "Bash" | "Skill" => {
            let command = tool_call
                .input
                .get("command")
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

            // Prefer structured deliverables; keep bash text artifact only when no path cards.
            if out.is_empty() && tool_call.name == "Bash" {
                let mut metadata = HashMap::new();
                metadata.insert("command".to_string(), serde_json::Value::String(command));
                metadata.insert("exit_code".to_string(), exit_code);

                let mut combined = String::new();
                if !stdout.is_empty() {
                    combined.push_str("== stdout ==\n");
                    combined.push_str(stdout);
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
                    kind: Some("bash".into()),
                    mime: None,
                    title: None,
                    bytes: None,
                    preview_path: None,
                    inline: Some(false),
                });
            }

            // Skill stdout path scan: fallback only when no structured emit.
            if tool_call.name == "Skill" && out.is_empty() {
                for token in stdout.split_whitespace() {
                    let cleaned = token.trim_matches(|c: char| {
                        c == '"' || c == '\'' || c == ',' || c == ')' || c == '('
                    });
                    let looks_deliverable = cleaned.ends_with(".pptx")
                        || cleaned.ends_with(".pdf")
                        || cleaned.ends_with(".png")
                        || cleaned.ends_with(".jpg")
                        || cleaned.ends_with(".jpeg")
                        || cleaned.ends_with(".webp")
                        || cleaned.ends_with(".docx")
                        || cleaned.ends_with(".xlsx")
                        || cleaned.ends_with(".csv")
                        || cleaned.ends_with(".mp4")
                        || cleaned.ends_with(".md");
                    if Path::new(cleaned).is_absolute() && looks_deliverable {
                        push_unique(&mut out, enrich_path_artifact(Artifact::from_path(cleaned)));
                    }
                }
            }
        }
        "GenerateImage" | "GenerateVideo" => {
            if let Some(path) = tool_output
                .result
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
            {
                let mut art = Artifact::from_path(path);
                if tool_call.name == "GenerateVideo" {
                    art.kind = Some("video".into());
                    art.name = "video".into();
                }
                art.inline = Some(true);
                push_unique(&mut out, enrich_path_artifact(art));
            }
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
        assert!(s.chars().count() < 20);
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

    #[test]
    fn extract_structured_artifacts_field() {
        let tc = ToolCall {
            id: "1".into(),
            name: "Skill".into(),
            input: json!({}),
        };
        let out = ToolOutput {
            result: json!({
                "stdout": "done",
                "artifacts": [{ "path": "/tmp/cover.png", "title": "封面" }]
            }),
            error: None,
            duration_ms: 1,
        };
        let arts = extract_artifacts(&tc, &out);
        assert_eq!(arts.len(), 1);
        assert_eq!(arts[0].resolved_kind(), "image");
        assert_eq!(arts[0].title.as_deref(), Some("封面"));
        assert_eq!(arts[0].inline, Some(true));
    }

    #[test]
    fn extract_stdout_footer() {
        let tc = ToolCall {
            id: "1".into(),
            name: "Skill".into(),
            input: json!({}),
        };
        let out = ToolOutput {
            result: json!({
                "stdout": "wrote file\nANYCODE_ARTIFACT:{\"path\":\"/tmp/deck.pptx\",\"kind\":\"presentation\"}\n"
            }),
            error: None,
            duration_ms: 1,
        };
        let arts = extract_artifacts(&tc, &out);
        assert!(!arts.is_empty());
        assert_eq!(arts[0].resolved_kind(), "presentation");
    }
}
