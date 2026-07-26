//! Heuristic user-facing progress updates (no reasoning exposure).

use anycode_core::LiveTraceEvent;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProgressPhase {
    Intent,
    Execute,
    Discovery,
    Deliver,
}

impl ProgressPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Intent => "intent",
            Self::Execute => "execute",
            Self::Discovery => "discovery",
            Self::Deliver => "deliver",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkStage {
    Inspect,
    Analyze,
    Implement,
    Verify,
}

impl WorkStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inspect => "inspect",
            Self::Analyze => "analyze",
            Self::Implement => "implement",
            Self::Verify => "verify",
        }
    }
}

pub(crate) fn infer_work_stage(tool_names: &[String]) -> Option<WorkStage> {
    if tool_names.is_empty() {
        return None;
    }
    let lower: Vec<String> = tool_names.iter().map(|n| n.to_ascii_lowercase()).collect();
    if lower.iter().any(|n| {
        matches!(
            n.as_str(),
            "write" | "edit" | "strreplace" | "applypatch" | "notebookedit" | "filewrite"
        )
    }) {
        return Some(WorkStage::Implement);
    }
    if lower
        .iter()
        .any(|n| matches!(n.as_str(), "bash" | "shell" | "run_terminal_cmd"))
    {
        return Some(WorkStage::Verify);
    }
    if lower.iter().any(|n| {
        matches!(
            n.as_str(),
            "glob" | "grep" | "read" | "fileread" | "webfetch" | "websearch"
        )
    }) {
        return Some(WorkStage::Inspect);
    }
    Some(WorkStage::Analyze)
}

fn first_sentence(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let end = trimmed
        .char_indices()
        .find(|(i, c)| *i > 0 && ['.', '。', '!', '！', '?', '？', '\n'].contains(c))
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(trimmed.len());
    trimmed[..end].trim().to_string()
}

fn tool_evidence_ref(_user_turn: u32, tool_turn: u32, idx: u32) -> String {
    format!("{tool_turn}:{idx}")
}

pub(crate) fn build_intent_update(turn: u32, seq: u32, text: &str) -> LiveTraceEvent {
    LiveTraceEvent::ProgressUpdate {
        turn,
        seq,
        phase: ProgressPhase::Intent.as_str().into(),
        work_stage: None,
        summary: first_sentence(text),
        next: None,
        discovery: None,
        evidence_refs: vec![],
    }
}

/// User-facing progress for a tool round (supports empty assistant text).
pub(crate) fn build_tool_round_progress(
    turn: u32,
    seq: u32,
    text: &str,
    tool_names: &[String],
    tool_turn: u32,
    tool_indices: &[u32],
    prefer_intent: bool,
) -> LiveTraceEvent {
    if prefer_intent {
        let summary = {
            let s = first_sentence(text);
            if s.is_empty() {
                if tool_names.is_empty() {
                    "正在开始执行任务".to_string()
                } else {
                    format!("正在开始：{}", tool_names.join("、"))
                }
            } else {
                s
            }
        };
        let next = tool_names.first().map(|name| format!("接着运行 {name}"));
        let evidence_refs: Vec<String> = tool_indices
            .iter()
            .map(|idx| format!("{tool_turn}:{idx}"))
            .collect();
        return LiveTraceEvent::ProgressUpdate {
            turn,
            seq,
            phase: ProgressPhase::Intent.as_str().into(),
            work_stage: infer_work_stage(tool_names).map(|s| s.as_str().to_string()),
            summary,
            next,
            discovery: None,
            evidence_refs,
        };
    }
    build_execute_update(turn, seq, text, tool_names, tool_turn, tool_indices)
}

pub(crate) fn build_execute_update(
    turn: u32,
    seq: u32,
    text: &str,
    tool_names: &[String],
    tool_turn: u32,
    tool_indices: &[u32],
) -> LiveTraceEvent {
    let summary = first_sentence(text);
    let summary = if summary.is_empty() {
        if let Some(name) = tool_names.first() {
            format!("正在运行 {name}")
        } else {
            "正在执行工具".to_string()
        }
    } else {
        summary
    };
    let next = tool_names.first().map(|name| format!("接着运行 {name}"));
    let evidence_refs: Vec<String> = tool_indices
        .iter()
        .map(|idx| format!("{tool_turn}:{idx}"))
        .collect();
    LiveTraceEvent::ProgressUpdate {
        turn,
        seq,
        phase: ProgressPhase::Execute.as_str().into(),
        work_stage: infer_work_stage(tool_names).map(|s| s.as_str().to_string()),
        summary,
        next,
        discovery: None,
        evidence_refs,
    }
}

pub(crate) fn build_discovery_from_failure(
    turn: u32,
    seq: u32,
    tool_name: &str,
    error: &str,
    user_turn: u32,
    tool_turn: u32,
    tool_idx: u32,
) -> LiveTraceEvent {
    let err_snippet: String = error.chars().take(120).collect();
    LiveTraceEvent::ProgressUpdate {
        turn,
        seq,
        phase: ProgressPhase::Discovery.as_str().into(),
        work_stage: Some(WorkStage::Verify.as_str().to_string()),
        summary: format!("{tool_name} 失败"),
        next: Some("正在检查错误输出并调整方案".to_string()),
        discovery: Some(if err_snippet.is_empty() {
            format!("{tool_name} 未成功完成，需要调整后续步骤")
        } else {
            err_snippet
        }),
        evidence_refs: vec![tool_evidence_ref(user_turn, tool_turn, tool_idx)],
    }
}

pub(crate) fn build_deliver_update(turn: u32, seq: u32, text: &str) -> LiveTraceEvent {
    LiveTraceEvent::ProgressUpdate {
        turn,
        seq,
        phase: ProgressPhase::Deliver.as_str().into(),
        work_stage: None,
        summary: first_sentence(text),
        next: None,
        discovery: None,
        evidence_refs: vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_work_stage_from_tool_names() {
        assert_eq!(
            infer_work_stage(&["Read".into(), "Grep".into()]),
            Some(WorkStage::Inspect)
        );
        assert_eq!(
            infer_work_stage(&["Write".into()]),
            Some(WorkStage::Implement)
        );
        assert_eq!(infer_work_stage(&["Bash".into()]), Some(WorkStage::Verify));
    }

    #[test]
    fn build_tool_round_progress_empty_text_first_turn() {
        let evt =
            build_tool_round_progress(1, 1, "", &["Glob".into(), "Read".into()], 1, &[1, 2], true);
        match evt {
            LiveTraceEvent::ProgressUpdate { summary, phase, .. } => {
                assert_eq!(phase, "intent");
                assert!(summary.contains("Glob"));
            }
            _ => panic!("expected ProgressUpdate"),
        }
    }

    #[test]
    fn build_execute_update_uses_summary() {
        let evt = build_execute_update(2, 1, "先检查测试。", &["Glob".into()], 2, &[1]);
        match evt {
            LiveTraceEvent::ProgressUpdate { summary, phase, .. } => {
                assert_eq!(phase, "execute");
                assert!(summary.contains("检查"));
            }
            _ => panic!("expected ProgressUpdate"),
        }
    }
}
