//! 已移除的全屏 ratatui 主循环与流式 REPL（[`crate::tasks::tasks_repl::run_interactive_tty_stream`]）执行态差异备忘（仅档案）。
//!
//! | 维度 | 历史全屏主循环 | 流式 `run_interactive_tty_stream` |
//! |------|----------------|-------------------------------------|
//! | `executing_since` 置位 | 多事件路径 | 自然语言 `SpawnNatural` 分支 |
//! | 执行中 transcript | `exec_live_tail` 等 | `build_stream_turn_plain(..., live: true)` + `turn_transcript_anchor` 截断 |
//! | 回合结束清理 | `consume_finished_turn` | `finish_stream_spawned_turn` + 回放 `build_stream_turn_plain` |
//!
//! 回归策略：对同一组 `messages` 快照，断言 `build_stream_turn_plain` 与流式路径一致；见下方 smoke 单测。

#[cfg(test)]
mod tests {
    use crate::repl::{drain_stream_repl_render_scrollback, StreamReplRenderMsg};
    use crate::term::transcript::build_stream_turn_plain;
    use anycode_core::{Message, MessageContent, MessageRole};
    use chrono::Utc;
    use std::collections::HashMap;
    use uuid::Uuid;

    /// 流式回合「空 messages」与渲染通道 drain 同路径可编译运行（提交前 smoke）。
    #[test]
    fn stream_repl_plain_build_and_render_drain_smoke() {
        let empty: Vec<Message> = vec![];
        assert_eq!(build_stream_turn_plain(0, &empty, 80, false), "");
        assert_eq!(build_stream_turn_plain(0, &empty, 80, true), "");

        let (tx, rx) = std::sync::mpsc::channel();
        tx.send(StreamReplRenderMsg::ScrollbackChunk("x".into()))
            .unwrap();
        tx.send(StreamReplRenderMsg::DockInvalidate).unwrap();
        tx.send(StreamReplRenderMsg::ClearScrollback).unwrap();
        tx.send(StreamReplRenderMsg::ScrollbackChunk("y".into()))
            .unwrap();
        let mut staging = Vec::new();
        drain_stream_repl_render_scrollback(&rx, &mut staging);
        assert_eq!(staging, vec!["y".to_string()]);

        let u = Message {
            id: Uuid::new_v4(),
            role: MessageRole::User,
            content: MessageContent::Text("hi".into()),
            timestamp: Utc::now(),
            metadata: HashMap::new(),
        };
        let msgs = vec![u];
        let waiting = build_stream_turn_plain(1, &msgs, 40, true);
        assert!(
            !waiting.contains("hi"),
            "流式执行中主区隐藏末条 user（与 stream_plain 用例一致），got {waiting:?}"
        );
        let idle = build_stream_turn_plain(1, &msgs, 40, false);
        assert!(
            idle.contains("hi"),
            "回合 idle 后应出现 user 行，got {idle:?}"
        );
    }
}
