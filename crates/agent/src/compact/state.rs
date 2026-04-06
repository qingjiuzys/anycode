//! 压缩管线可扩展状态：当前从会话消息推导（未来可由工具执行路径增量写入 `SessionCompactionState`）。

use anycode_core::prelude::Message;

pub use super::snippets::FileReadSnippet;

/// 一次压缩周期内的派生状态；默认每次 `compact_session_messages` 在栈上新建并 `refresh_from_messages`。
#[derive(Debug, Default, Clone)]
pub struct SessionCompactionState {
    pub file_reads: Vec<FileReadSnippet>,
}

impl SessionCompactionState {
    /// 从完整会话扫描 FileRead 工具结果（与 Claude `readFileState` 缓存互补的纯消息路径）。
    pub fn refresh_from_messages(&mut self, session: &[Message]) {
        self.file_reads = super::snippets::collect_from_session(session);
    }

    pub fn clear(&mut self) {
        self.file_reads.clear();
    }
}
