//! Dashboard / pipe REPL text-file injection (`@anycode/text-file:` line protocol).

use std::path::{Path, PathBuf};

pub const TEXT_FILE_LINE_PREFIX: &str = "@anycode/text-file:";

#[must_use]
pub fn parse_text_file_line(line: &str) -> Option<PathBuf> {
    line.trim()
        .strip_prefix(TEXT_FILE_LINE_PREFIX)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

/// Prefix the user prompt with a readable reference to uploaded text files.
pub fn augment_prompt_with_text_files(prompt: &str, paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return prompt.to_string();
    }
    let mut header = String::from("Attached reference file(s) — use the Read tool to access:\n");
    for path in paths {
        let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
        header.push_str(&format!("- @file:{} ({name})\n", path.display()));
    }
    if prompt.trim().is_empty() {
        header.trim_end().to_string()
    } else {
        format!("{header}\n{prompt}")
    }
}

pub fn remove_text_files(paths: &[PathBuf]) {
    for path in paths {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_text_file_line_extracts_path() {
        let line = "@anycode/text-file:/tmp/demo.txt\n";
        assert_eq!(
            parse_text_file_line(line),
            Some(PathBuf::from("/tmp/demo.txt"))
        );
    }

    #[test]
    fn parse_text_file_line_rejects_invalid() {
        assert_eq!(parse_text_file_line("hello"), None);
        assert_eq!(parse_text_file_line("@anycode/text-file:"), None);
    }

    #[test]
    fn augment_prompt_includes_file_hints() {
        let paths = vec![PathBuf::from("/tmp/notes.md")];
        let out = augment_prompt_with_text_files("summarize", &paths);
        assert!(out.contains("@file:/tmp/notes.md"));
        assert!(out.contains("summarize"));
    }
}
