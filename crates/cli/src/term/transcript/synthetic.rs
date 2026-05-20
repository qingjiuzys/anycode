//! Synthetic transcript fixtures for terminal load-model measurements (see docs/terminal-load-model.md).

use super::types::TranscriptEntry;

/// Build `count` repetitive tool-turn entries for scroll / layout benchmarks.
pub(crate) fn synthetic_tool_turn_entries(count: usize) -> Vec<TranscriptEntry> {
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        out.push(TranscriptEntry::ToolTurn {
            fold_id: i as u64 + 1,
            name: "FileRead".into(),
            args: format!(r#"{{"file_path":"/bench/file_{i}.rs"}}"#),
            tool_use_id: format!("tool-{i}"),
            tool_name: Some("FileRead".into()),
            body: format!("fn bench_{i}() {{ /* synthetic */ }}\n"),
            is_error: false,
        });
        if i % 7 == 0 {
            out.push(TranscriptEntry::AssistantMarkdown(format!(
                "Reviewed chunk {i} in synthetic transcript."
            )));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{synthetic_tool_turn_entries, TranscriptEntry};

    #[test]
    fn generator_reaches_at_least_requested_tool_turns() {
        let entries = synthetic_tool_turn_entries(100);
        let tool_turns = entries
            .iter()
            .filter(|e| matches!(e, TranscriptEntry::ToolTurn { .. }))
            .count();
        assert_eq!(tool_turns, 100);
    }

    #[test]
    fn tier_s_fixture_is_bounded() {
        let entries = synthetic_tool_turn_entries(128);
        assert!(entries.len() >= 128);
        assert!(entries.len() < 256);
    }

    #[test]
    fn tier_s_transcript_pipeline_completes_quickly() {
        use super::super::pipeline::apply_tool_transcript_pipeline;
        let mut entries = synthetic_tool_turn_entries(128);
        let start = std::time::Instant::now();
        let mut fold_id = 1u64;
        apply_tool_transcript_pipeline(&mut entries, &mut fold_id);
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
        assert!(!entries.is_empty());
    }

    #[test]
    fn tier_m_transcript_pipeline_completes() {
        use super::super::pipeline::apply_tool_transcript_pipeline;
        let mut entries = synthetic_tool_turn_entries(512);
        let mut fold_id = 1u64;
        apply_tool_transcript_pipeline(&mut entries, &mut fold_id);
        assert!(!entries.is_empty());
    }

    /// Tier L proxy (~8k tool turns). Run with `cargo test tier_l -- --ignored`.
    /// Runtime virtual-scroll Tier S/M tests live in `repl/stream_viewport.rs`.
    #[test]
    #[ignore = "tier L load benchmark; run in nightly or with ANYCODE_BENCH=1"]
    fn tier_l_transcript_pipeline_degrades_gracefully() {
        if std::env::var_os("ANYCODE_BENCH").is_none() {
            // Allow `cargo test -- --ignored` without env var.
        }
        use super::super::pipeline::apply_tool_transcript_pipeline;
        let mut entries = synthetic_tool_turn_entries(8192);
        let start = std::time::Instant::now();
        let mut fold_id = 1u64;
        apply_tool_transcript_pipeline(&mut entries, &mut fold_id);
        assert!(start.elapsed() < std::time::Duration::from_secs(30));
        assert!(!entries.is_empty());
    }
}
