#[cfg(test)]
mod tests {
    use crate::tui::transcript::{
        assistant_markdown_meaningful_eq, coalesce_read_tool_batches, collapse_tool_groups,
        ctrl_o_fold_cycle, layout_workspace, message_to_entries, transcript_tail_closing_matches,
        CollapsibleToolBlock, TranscriptEntry, WorkspaceLiveLayout,
    };
    use anycode_core::{
        Message, MessageContent, MessageRole, ANYCODE_CONTEXT_USER_METADATA_KEY,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn message_to_entries_skips_injected_context_user_messages() {
        let mut meta = HashMap::new();
        meta.insert(
            ANYCODE_CONTEXT_USER_METADATA_KEY.to_string(),
            serde_json::json!(true),
        );
        let tagged = Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("## Workflow\nx".into()),
            timestamp: Utc::now(),
            metadata: meta,
        };
        assert!(message_to_entries(&tagged).is_empty());

        let legacy = Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("## Model Routing\nKnown aliases: a, b".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        assert!(message_to_entries(&legacy).is_empty());

        let real = Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("分析下当前项目".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        assert_eq!(message_to_entries(&real).len(), 1);
    }

    #[test]
    fn assistant_markdown_meaningful_eq_unwraps_candidate_json() {
        let stored = "{\"content\":\"summary\"}";
        let candidate = "{\"content\":\"summary\"}";
        assert!(assistant_markdown_meaningful_eq(stored, candidate));
    }

    #[test]
    fn transcript_tail_closing_matches_skips_collapsed_tool_group() {
        let entries = vec![
            TranscriptEntry::AssistantMarkdown("hi".into()),
            TranscriptEntry::CollapsedToolGroup {
                fold_id: 1,
                blocks: vec![],
            },
        ];
        assert!(transcript_tail_closing_matches(&entries, "hi"));
        assert!(!transcript_tail_closing_matches(&entries, "bye"));
    }

    #[test]
    fn coalesce_merges_consecutive_file_read_turns() {
        let mut entries = vec![
            TranscriptEntry::ToolTurn {
                fold_id: 1,
                name: "FileRead".into(),
                args: r#"{"file_path":"/a"}"#.into(),
                tool_use_id: "u1".into(),
                tool_name: Some("FileRead".into()),
                body: "body1".into(),
                is_error: false,
            },
            TranscriptEntry::ToolTurn {
                fold_id: 2,
                name: "FileRead".into(),
                args: r#"{"file_path":"/b"}"#.into(),
                tool_use_id: "u2".into(),
                tool_name: Some("FileRead".into()),
                body: "body2".into(),
                is_error: false,
            },
        ];
        let mut next = 100u64;
        coalesce_read_tool_batches(&mut entries, &mut next);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            TranscriptEntry::ReadToolBatch { fold_id, parts } => {
                assert_eq!(*fold_id, 101);
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0].1, "body1");
                assert_eq!(parts[1].1, "body2");
            }
            _ => unreachable!("test: expected ReadToolBatch"),
        }
    }

    #[test]
    fn coalesce_single_file_read_stays_tool_turn() {
        let mut entries = vec![TranscriptEntry::ToolTurn {
            fold_id: 7,
            name: "FileRead".into(),
            args: "{}".into(),
            tool_use_id: "u1".into(),
            tool_name: None,
            body: "x".into(),
            is_error: false,
        }];
        let mut next = 100u64;
        coalesce_read_tool_batches(&mut entries, &mut next);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            TranscriptEntry::ToolTurn { fold_id, .. } => assert_eq!(*fold_id, 7),
            _ => unreachable!("test: expected ToolTurn"),
        }
    }

    #[test]
    fn coalesce_does_not_merge_file_read_separated_by_other_tool() {
        let mut entries = vec![
            TranscriptEntry::ToolTurn {
                fold_id: 1,
                name: "FileRead".into(),
                args: "{}".into(),
                tool_use_id: "u1".into(),
                tool_name: None,
                body: "a".into(),
                is_error: false,
            },
            TranscriptEntry::ToolTurn {
                fold_id: 2,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "u2".into(),
                tool_name: None,
                body: "out".into(),
                is_error: false,
            },
            TranscriptEntry::ToolTurn {
                fold_id: 3,
                name: "FileRead".into(),
                args: "{}".into(),
                tool_use_id: "u3".into(),
                tool_name: None,
                body: "b".into(),
                is_error: false,
            },
        ];
        let mut next = 100u64;
        coalesce_read_tool_batches(&mut entries, &mut next);
        assert_eq!(entries.len(), 3);
    }

    #[test]
    fn ctrl_o_first_press_expands_latest_fold() {
        let entries = vec![
            TranscriptEntry::ToolTurn {
                fold_id: 11,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "u1".into(),
                tool_name: None,
                body: "a".into(),
                is_error: false,
            },
            TranscriptEntry::ToolTurn {
                fold_id: 12,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "u2".into(),
                tool_name: None,
                body: "b".into(),
                is_error: false,
            },
        ];
        let mut expanded = std::collections::HashSet::new();
        ctrl_o_fold_cycle(&entries, &mut expanded);
        assert_eq!(expanded.len(), 1);
        assert!(expanded.contains(&12));
    }

    #[test]
    fn ctrl_o_second_press_collapses_all_expanded_folds() {
        let entries = vec![TranscriptEntry::ToolTurn {
            fold_id: 99,
            name: "Bash".into(),
            args: "{}".into(),
            tool_use_id: "u99".into(),
            tool_name: None,
            body: "out".into(),
            is_error: false,
        }];
        let mut expanded = std::collections::HashSet::new();
        ctrl_o_fold_cycle(&entries, &mut expanded);
        assert!(expanded.contains(&99));
        ctrl_o_fold_cycle(&entries, &mut expanded);
        assert!(expanded.is_empty());
    }

    #[test]
    fn collapse_merges_consecutive_bash_turns() {
        let mut entries = vec![
            TranscriptEntry::ToolTurn {
                fold_id: 1,
                name: "Bash".into(),
                args: r#"{"command":"ls"}"#.into(),
                tool_use_id: "a".into(),
                tool_name: None,
                body: "out".into(),
                is_error: false,
            },
            TranscriptEntry::ToolTurn {
                fold_id: 2,
                name: "Bash".into(),
                args: r#"{"command":"pwd"}"#.into(),
                tool_use_id: "b".into(),
                tool_name: None,
                body: "out2".into(),
                is_error: false,
            },
        ];
        let mut next = 200u64;
        collapse_tool_groups(&mut entries, &mut next);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            TranscriptEntry::CollapsedToolGroup { fold_id, blocks } => {
                assert_eq!(*fold_id, 201);
                assert_eq!(blocks.len(), 2);
            }
            _ => unreachable!("test: expected CollapsedToolGroup"),
        }
    }

    #[test]
    fn collapse_single_bash_stays_tool_turn() {
        let mut entries = vec![TranscriptEntry::ToolTurn {
            fold_id: 9,
            name: "Bash".into(),
            args: "{}".into(),
            tool_use_id: "a".into(),
            tool_name: None,
            body: "x".into(),
            is_error: false,
        }];
        let mut next = 300u64;
        collapse_tool_groups(&mut entries, &mut next);
        assert_eq!(entries.len(), 1);
        match &entries[0] {
            TranscriptEntry::ToolTurn { fold_id, .. } => assert_eq!(*fold_id, 9),
            _ => unreachable!("test: expected ToolTurn"),
        }
    }

    #[test]
    fn collapse_assistant_breaks_group() {
        let mut entries = vec![
            TranscriptEntry::ToolTurn {
                fold_id: 1,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "a".into(),
                tool_name: None,
                body: "o1".into(),
                is_error: false,
            },
            TranscriptEntry::AssistantMarkdown("summary".into()),
            TranscriptEntry::ToolTurn {
                fold_id: 2,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "b".into(),
                tool_name: None,
                body: "o2".into(),
                is_error: false,
            },
        ];
        let mut next = 400u64;
        collapse_tool_groups(&mut entries, &mut next);
        assert_eq!(entries.len(), 3);
        assert!(matches!(entries[0], TranscriptEntry::ToolTurn { .. }));
        assert!(matches!(entries[1], TranscriptEntry::AssistantMarkdown(_)));
        assert!(matches!(entries[2], TranscriptEntry::ToolTurn { .. }));
    }

    #[test]
    fn layout_collapsed_summary_running_when_executing() {
        let blocks = vec![
            CollapsibleToolBlock::Turn {
                fold_id: 1,
                name: "Bash".into(),
                args: r#"{"command":"npm test"}"#.into(),
                tool_use_id: "a".into(),
                tool_name: None,
                body: "".into(),
                is_error: false,
            },
            CollapsibleToolBlock::Turn {
                fold_id: 2,
                name: "Bash".into(),
                args: r#"{"command":"ls"}"#.into(),
                tool_use_id: "b".into(),
                tool_name: None,
                body: "".into(),
                is_error: false,
            },
        ];
        let entries = vec![TranscriptEntry::CollapsedToolGroup {
            fold_id: 42,
            blocks,
        }];
        let folds = std::collections::HashSet::new();
        let lines = layout_workspace(
            &entries,
            100,
            &folds,
            WorkspaceLiveLayout {
                executing: true,
                working_elapsed_secs: Some(3),
                ..Default::default()
            },
        );
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(
            joined.contains("Running") || joined.contains("正在执行"),
            "expected active bash phrasing, got {joined}"
        );
        assert!(
            joined.contains('3') && (joined.contains('s') || joined.contains('秒')),
            "expected elapsed seconds hint, got {joined}"
        );
        assert!(joined.contains('…'));
    }

    #[test]
    fn layout_collapsed_summary_ran_when_idle() {
        let blocks = vec![
            CollapsibleToolBlock::Turn {
                fold_id: 1,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "a".into(),
                tool_name: None,
                body: "".into(),
                is_error: false,
            },
            CollapsibleToolBlock::Turn {
                fold_id: 2,
                name: "Bash".into(),
                args: "{}".into(),
                tool_use_id: "b".into(),
                tool_name: None,
                body: "".into(),
                is_error: false,
            },
        ];
        let entries = vec![TranscriptEntry::CollapsedToolGroup {
            fold_id: 42,
            blocks,
        }];
        let folds = std::collections::HashSet::new();
        let lines = layout_workspace(&entries, 100, &folds, WorkspaceLiveLayout::default());
        let joined: String = lines
            .iter()
            .flat_map(|l| l.spans.iter().map(|s| s.content.to_string()))
            .collect();
        assert!(
            joined.contains("Ran") || joined.contains("已执行"),
            "expected completed bash phrasing, got {joined}"
        );
        assert!(!joined.contains("Running") && !joined.contains("正在执行"));
    }
}
