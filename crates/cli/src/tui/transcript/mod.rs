//! Workspace 消息块与 Markdown 排版。
//!
//! Claude Code 对齐：`apply_tool_transcript_pipeline` 为工具展示唯一归并路径；`rebuild_live_turn_tail` / 收尾判定供 TUI 实时与 turn 结束共用。
#![allow(unused_imports)] // 门面 `pub(crate) use`：本文件不引用符号，供 `crate::tui::transcript::…` 统一路径

mod dump_message;
mod pipeline;
mod stream_plain;
mod tool_render;
mod types;
mod workspace_draw;

#[cfg(test)]
mod tests;

pub(crate) use dump_message::{
    last_nonempty_assistant_text, message_to_entries, rebuild_live_turn_tail,
    transcript_dump_plain_text, transcript_tail_closing_matches,
};
pub(crate) use pipeline::{
    apply_tool_transcript_pipeline, coalesce_read_tool_batches, collapse_tool_groups,
    ctrl_o_fold_cycle, normalize_transcript_global,
};
pub(crate) use stream_plain::build_stream_turn_plain;
pub(crate) use tool_render::assistant_markdown_meaningful_eq;
pub(crate) use types::{CollapsibleToolBlock, TranscriptEntry, WorkspaceLiveLayout};
pub(crate) use workspace_draw::layout_workspace;
