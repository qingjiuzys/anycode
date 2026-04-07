//! 与 Claude Code 一致：Shell 仅在 transcript 的 `⏺ Bash(…)` 中展示；
//! 回合末尾只列**落盘产物**（如 FileWrite），避免重复罗列 bash。

use anycode_core::Artifact;
use std::collections::BTreeMap;

/// TUI / `anycode run` 回合尾展示：仅 **FileWrite** 类落盘项；**不**列出 Bash（已在 transcript）。
pub fn claude_turn_written_lines(artifacts: &[Artifact]) -> Vec<String> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for a in artifacts {
        if a.name == "file" {
            if let Some(ref p) = a.path {
                if !p.is_empty() {
                    *counts.entry(p.clone()).or_insert(0) += 1;
                }
            }
        }
    }
    counts
        .into_iter()
        .map(|(p, n)| {
            if n > 1 {
                format!("FileWrite(write {p}) ×{n}")
            } else {
                format!("FileWrite(write {p})")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::Artifact;
    use std::collections::HashMap;

    fn bash() -> Artifact {
        Artifact {
            name: "bash".into(),
            path: None,
            content: None,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn bash_only_yields_empty_footer() {
        let v = vec![bash(), bash()];
        assert!(claude_turn_written_lines(&v).is_empty());
    }

    #[test]
    fn file_matches_transcript_one_liner() {
        let v = vec![Artifact {
            name: "file".into(),
            path: Some("/tmp/x".into()),
            content: None,
            metadata: HashMap::new(),
        }];
        assert_eq!(
            claude_turn_written_lines(&v),
            vec!["FileWrite(write /tmp/x)".to_string()]
        );
    }

    #[test]
    fn mixed_bash_and_file_lists_only_file() {
        let v = vec![
            bash(),
            Artifact {
                name: "file".into(),
                path: Some("src/lib.rs".into()),
                content: None,
                metadata: HashMap::new(),
            },
            bash(),
        ];
        assert_eq!(
            claude_turn_written_lines(&v),
            vec!["FileWrite(write src/lib.rs)".to_string()]
        );
    }

    #[test]
    fn duplicate_file_writes_are_collapsed_with_count() {
        let v = vec![
            Artifact {
                name: "file".into(),
                path: Some("src/lib.rs".into()),
                content: None,
                metadata: HashMap::new(),
            },
            Artifact {
                name: "file".into(),
                path: Some("src/lib.rs".into()),
                content: None,
                metadata: HashMap::new(),
            },
        ];
        assert_eq!(
            claude_turn_written_lines(&v),
            vec!["FileWrite(write src/lib.rs) ×2".to_string()]
        );
    }
}
