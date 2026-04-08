//! Transcript 语义类型：`TranscriptEntry`、`CollapsibleToolBlock`、实时布局元数据。

use ratatui::text::Line;

/// 与 Claude `collapsed_read_search` 对齐：合并展示用的子块（由连续可折叠 `ToolTurn` / `ReadToolBatch` 压平而来）。
#[derive(Clone, Debug)]
pub(crate) enum CollapsibleToolBlock {
    Turn {
        fold_id: u64,
        name: String,
        args: String,
        #[allow(dead_code)] // 与 ToolTurn 对齐保留，折叠 UI 当前不展示 id
        tool_use_id: String,
        tool_name: Option<String>,
        body: String,
        is_error: bool,
    },
    ReadBatch {
        fold_id: u64,
        parts: Vec<(String, String, bool)>,
    },
}

/// 绘制时「是否仍在跑 turn」：与 Claude `isActiveGroup` + `inProgressToolUseIDs` 同目的。
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WorkspaceLiveLayout {
    pub executing: bool,
    /// 与顶栏一致：整秒，用于长 bash 时在摘要后挂 `(Ns)`（对齐 `bash_progress` ≥2s）。
    pub working_elapsed_secs: Option<u64>,
    /// 与 Buddy 同节拍（约 250ms×4），用于活动行 `⏺` 呼吸闪烁。
    pub pulse_frame: u64,
}

/// Workspace 语义块：按终端宽度排版为物理行后再滚动（与 ratatui 自动换行脱钩）。
#[derive(Clone)]
pub(crate) enum TranscriptEntry {
    User(String),
    AssistantMarkdown(String),
    ToolCall {
        tool_use_id: String,
        name: String,
        args: String,
    },
    ToolResult {
        tool_use_id: String,
        /// 来自 `Message.metadata["tool_name"]`（若有则顶栏显示工具名而非冗长 id）。
        tool_name: Option<String>,
        body: String,
        is_error: bool,
    },
    /// 已归并的「调用 + 结果」，支持折叠展示（对齐 Claude Code）。
    ToolTurn {
        fold_id: u64,
        name: String,
        args: String,
        #[allow(dead_code)] // 折叠块与后续 tool_result 关联用，当前渲染未读
        tool_use_id: String,
        tool_name: Option<String>,
        body: String,
        is_error: bool,
    },
    /// 同一轮内多次 `FileRead` 合并展示（Claude：`Read N files (ctrl+o to expand)`）。
    ReadToolBatch {
        fold_id: u64,
        /// 每项：`args` JSON、`body` 原文、`is_error`
        parts: Vec<(String, String, bool)>,
    },
    /// 连续 Read/Grep/Glob/Bash 等合并为一条摘要（Claude：`collapsed_read_search`）。
    CollapsedToolGroup {
        fold_id: u64,
        blocks: Vec<CollapsibleToolBlock>,
    },
    Plain(Vec<Line<'static>>),
}
