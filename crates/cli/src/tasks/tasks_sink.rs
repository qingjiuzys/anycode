//! REPL / `run` 共享输出目标（stdio vs 流式 Inline transcript）。

use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::repl_inline::ReplLineState;

/// REPL 输出目标：管道/工作流走真实终端；Stream 写入 [`ReplLineState::transcript`]（由 ratatui Inline 视口展示）。
pub(crate) enum ReplSink {
    Stdio,
    Stream {
        state: Arc<Mutex<ReplLineState>>,
        tail: String,
    },
}

impl ReplSink {
    pub(crate) fn line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        match self {
            ReplSink::Stdio => println!("{s}"),
            ReplSink::Stream { state, .. } => {
                let st = state.lock().unwrap_or_else(|e| e.into_inner());
                let mut t = st.transcript.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(s);
                t.push('\n');
            }
        }
    }

    /// 与 `eprintln!` 对齐的 stderr 行；`Stream` 仍走真实 stderr。
    pub(crate) fn eprint_line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        match self {
            ReplSink::Stdio => eprintln!("{s}"),
            ReplSink::Stream { .. } => {
                eprintln!("{s}");
            }
        }
    }

    pub(crate) fn push_stdout_str(&mut self, s: &str) {
        match self {
            ReplSink::Stdio => {
                print!("{s}");
                let _ = std::io::stdout().flush();
            }
            ReplSink::Stream { state, tail } => {
                tail.push_str(s);
                loop {
                    let Some(i) = tail.find('\n') else {
                        break;
                    };
                    let line = tail[..i].to_string();
                    tail.drain(..i + 1);
                    let st = state.lock().unwrap_or_else(|e| e.into_inner());
                    let mut t = st.transcript.lock().unwrap_or_else(|e| e.into_inner());
                    t.push_str(&line);
                    t.push('\n');
                }
            }
        }
    }
}
