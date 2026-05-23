//! Structured gate and conversation lines for `output.log` / dashboard ingestion.

use crate::error::CoreError;
use crate::ids::TaskId;
use crate::task_output::DiskTaskOutput;

const MAX_LOG_TEXT_CHARS: usize = 8000;

/// Escape user/assistant text for a single log line (` text=` suffix).
#[must_use]
pub fn encode_log_text(text: &str) -> String {
    let truncated: String = text.chars().take(MAX_LOG_TEXT_CHARS).collect();
    truncated
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "")
        .replace('\t', "\\t")
}

/// Decode text from a log line ` text=` suffix.
#[must_use]
pub fn decode_log_text(encoded: &str) -> String {
    let mut out = String::with_capacity(encoded.len());
    let mut chars = encoded.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Format `[user_prompt]` for dashboard conversation replay.
#[must_use]
pub fn format_user_prompt_log_line(prompt: &str) -> String {
    format!("[user_prompt] text={}", encode_log_text(prompt))
}

/// Format `[assistant_response]` for dashboard conversation replay.
#[must_use]
pub fn format_assistant_response_log_line(turn: usize, text: &str) -> String {
    format!(
        "[assistant_response] turn={turn} text={}",
        encode_log_text(text)
    )
}

/// Format a single `[gate]` log line (parsed by `anycode-dashboard` log_parser).
#[must_use]
pub fn format_gate_log_line(name: &str, status: &str, command: &str, output: &str) -> String {
    let mut out: String = output
        .chars()
        .filter(|c| *c != '\n' && *c != '\r')
        .collect();
    if out.len() > 400 {
        out.truncate(400);
        out.push('…');
    }
    format!("[gate] name={name} status={status} command={command} output={out}")
}

/// Append a gate line to `output.log` when disk logging is enabled.
pub fn append_gate_log(
    disk: &DiskTaskOutput,
    task_id: TaskId,
    name: &str,
    status: &str,
    command: &str,
    output: &str,
) -> Result<(), CoreError> {
    let line = format_gate_log_line(name, status, command, output);
    disk.append_line(task_id, &line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_log_text() {
        let raw = "hello\nworld\t!";
        let enc = encode_log_text(raw);
        assert_eq!(decode_log_text(&enc), raw);
    }

    #[test]
    fn user_prompt_line_parses() {
        let line = format_user_prompt_log_line("run tests");
        assert!(line.starts_with("[user_prompt] text="));
    }
}
