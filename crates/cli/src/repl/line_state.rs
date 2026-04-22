//! 流式 REPL 共享状态（与 ratatui 绘制、Tokio 回合循环共用）。

#![allow(dead_code)] // `TRANSCRIPT_MAX_DISPLAY_LINES` 由 `inline` transcript 辅助与单测读取。

use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::term::input::InputState;
use anycode_core::{Message, TurnTokenUsage};
use tokio::sync::Mutex as AsyncMutex;

/// 仅用于绘制：保留尾部若干行（旧 tail 裁剪路径；`repl_stream_transcript_bottom_padded` 与单测共用）。
pub(crate) const TRANSCRIPT_MAX_DISPLAY_LINES: usize = 256;

/// 流式 REPL 主区折行缓存（与 `claude-code-rust` 视口高度测量同思路）。
#[derive(Clone, Debug, Default)]
pub(crate) struct StreamTranscriptLayoutCache {
    pub key: u64,
    pub width: u16,
    pub total_rows: usize,
    /// `prefix_row[i]` = 全局显示行下标到第 `i` 条逻辑行起点。
    pub prefix_row: Vec<usize>,
    pub logical_heights: Vec<usize>,
}

pub(crate) struct ReplLineState {
    pub input: InputState,
    pub slash_pick: usize,
    pub slash_suppress: bool,
    pub input_history: Vec<String>,
    pub history_idx: Option<usize>,
    /// 流式 / Inline REPL 底栏顶行：模型 / 审批等（与全屏 TUI 脚标信息对齐）。
    pub dock_status: String,
    /// 底栏左列：`ctx` + `? help` + 可选滚动提示（与全屏脚标左半对齐）。
    pub dock_footer_left: String,
    /// 任务与 REPL 消息（显示在输入区上方）；与异步任务共享以便 tail 写入时重绘。
    pub transcript: Arc<Mutex<String>>,
    /// 流式 REPL 主区宽度（ratatui `draw` 回写），供 transcript 排版换行。
    pub stream_viewport_width: u16,
    /// 与全屏 TUI 一致：待处理的工具审批（仅流式 REPL 主循环设置）。
    pub pending_approval: Option<crate::term::PendingApproval>,
    pub pending_user_question: Option<crate::term::PendingUserQuestion>,
    pub approval_menu_selected: usize,
    pub user_question_menu_selected: usize,
    /// 流式 REPL：自然语言轮开始执行时起算，供 Prompt HUD 显示耗时（与全屏 TUI `executing_since` 一致）。
    pub executing_since: Option<Instant>,
    /// 回合结束后在 prompt 上方短暂显示 Claude 风格摘要（耗时 + ctx tokens）。
    pub finished_turn_summary: Option<String>,
    pub finished_turn_summary_until: Option<Instant>,
    /// 流式 REPL 主区（transcript）当前可视高度（行），每帧由 ratatui `draw` 回写；**0** 表示尚未绘制。
    /// 供 **PgUp / PgDn** 等与视口成比例的步长（「一页」= 本区高度）。
    pub stream_transcript_viewport_h: u16,
    /// 主区向上滚动的显示行数（从贴底算起，越大越「老」）；仅流式 Inline 使用。
    pub stream_transcript_scroll: usize,
    /// 与 `claude-code-rust` 一致：贴底时自动跟随新输出；用户上滚后为 `false`。
    pub stream_repl_auto_scroll_follow: bool,
    /// 平滑滚动位置（全局「显示行」坐标，浮点）。
    pub stream_repl_scroll_pos: f32,
    pub stream_repl_scroll_target: f32,
    /// 按视口宽度缓存的折行高度与前缀和（避免每帧 O(n) 全量折行）。
    pub stream_transcript_layout: StreamTranscriptLayoutCache,
    /// 最近完成的一轮 `execute_turn` 聚合用量（供 `/context` 与 HUD 对齐）。
    pub last_turn_token_usage: Option<TurnTokenUsage>,
    /// 退出时 `ANYCODE_TERM_EXIT_SCROLLBACK_DUMP=anchor` 用的字节偏移：当前「自然语言轮」写入前 `transcript.len()`（与异步侧 `turn_transcript_anchor` 一致）。
    pub stream_exit_dump_anchor: usize,
    /// `true` 时使用 `insert_before` 路径；备用屏全屏时为 `false`（无宿主 scrollback，改走 transcript）。
    pub stream_repl_host_scrollback: bool,
    /// 自然语言回合执行中：共享 `messages` 句柄，供轴心线程 [`crate::tasks::stream_repl_loop::tick_executing_stream_transcript`] 每帧 `try_lock` 刷新主区。
    pub stream_exec_messages: Option<Arc<AsyncMutex<Vec<Message>>>>,
    /// 与 `append_user_spawn_turn` 返回的 `prev` 一致，供 `build_stream_turn_plain(exec_prev_len, …)`。
    pub stream_exec_prev_len: usize,
    /// 本轮写入前 `transcript` 字节偏移（与 worker 侧 `turn_transcript_anchor` 同步）。
    pub stream_exec_transcript_anchor: usize,
}

impl Default for ReplLineState {
    fn default() -> Self {
        Self {
            input: InputState::default(),
            slash_pick: 0,
            slash_suppress: false,
            input_history: Vec::new(),
            history_idx: None,
            dock_status: String::new(),
            dock_footer_left: String::new(),
            transcript: Arc::new(Mutex::new(String::new())),
            stream_viewport_width: 80,
            pending_approval: None,
            pending_user_question: None,
            approval_menu_selected: 0,
            user_question_menu_selected: 0,
            executing_since: None,
            finished_turn_summary: None,
            finished_turn_summary_until: None,
            stream_transcript_viewport_h: 0,
            stream_transcript_scroll: 0,
            stream_repl_auto_scroll_follow: true,
            stream_repl_scroll_pos: 0.0,
            stream_repl_scroll_target: 0.0,
            stream_transcript_layout: StreamTranscriptLayoutCache::default(),
            last_turn_token_usage: None,
            stream_exit_dump_anchor: 0,
            stream_repl_host_scrollback: false,
            stream_exec_messages: None,
            stream_exec_prev_len: 0,
            stream_exec_transcript_anchor: 0,
        }
    }
}

pub(crate) enum ReplCtl {
    Continue,
    Submit(String),
    /// 与全屏 TUI Ctrl+L 一致：清空本会话消息并重建 system 上下文。
    ClearSession,
    /// 回合进行中（`executing_since`）：请求 `execute_turn_from_messages` 协作取消。
    CooperativeCancelTurn,
    Eof,
}

pub(crate) fn reset_slash_state(state: &mut ReplLineState) {
    state.slash_pick = 0;
    state.slash_suppress = false;
}

/// 新输入 / 回合结束 / 清会话：回到贴底跟随（与 `claude-code-rust` `auto_scroll` 一致）。
pub(crate) fn stream_repl_scroll_reset_to_bottom(state: &mut ReplLineState) {
    state.stream_transcript_scroll = 0;
    state.stream_repl_auto_scroll_follow = true;
    state.stream_repl_scroll_pos = 0.0;
    state.stream_repl_scroll_target = 0.0;
    state.stream_transcript_layout = StreamTranscriptLayoutCache::default();
}

/// 一页 = 当前 transcript 区高度（行）；首帧未绘制前回退 **8**（与旧版固定步长一致）。
pub(crate) fn stream_transcript_page_step(state: &ReplLineState) -> usize {
    if state.stream_transcript_viewport_h == 0 {
        8
    } else {
        state.stream_transcript_viewport_h as usize
    }
}

/// 滚轮步长：约为「一页」的 1/4，至少 1 行，随终端拉高而变大。
pub(crate) fn stream_transcript_wheel_step(state: &ReplLineState) -> usize {
    (stream_transcript_page_step(state) / 4).max(1)
}
