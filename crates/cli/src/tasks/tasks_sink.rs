//! REPL / `run` 共享输出目标（stdio vs 内嵌 TUI transcript）。

use std::io::Write;
use std::sync::{Arc, Mutex};

/// REPL 输出目标：管道/工作流走真实终端；TTY 全屏写入 transcript 并在每次增量后重绘。
pub(crate) enum ReplSink {
    Stdio,
    Tui {
        transcript: Arc<Mutex<String>>,
        on_flush: Arc<dyn Fn() + Send + Sync>,
    },
}

impl ReplSink {
    pub(crate) fn line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        match self {
            ReplSink::Stdio => println!("{s}"),
            ReplSink::Tui {
                transcript,
                on_flush,
            } => {
                let mut t = transcript.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(s);
                t.push('\n');
                drop(t);
                on_flush();
            }
        }
    }

    /// 与 `eprintln!` 对齐的 stderr 行；TTY 下仍进入 transcript（与原先一致）。
    pub(crate) fn eprint_line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        match self {
            ReplSink::Stdio => eprintln!("{s}"),
            ReplSink::Tui {
                transcript,
                on_flush,
            } => {
                let mut t = transcript.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(s);
                t.push('\n');
                drop(t);
                on_flush();
            }
        }
    }

    pub(crate) fn push_stdout_str(&mut self, s: &str) {
        match self {
            ReplSink::Stdio => {
                print!("{s}");
                let _ = std::io::stdout().flush();
            }
            ReplSink::Tui {
                transcript,
                on_flush,
            } => {
                let mut t = transcript.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(s);
                drop(t);
                on_flush();
            }
        }
    }
}
