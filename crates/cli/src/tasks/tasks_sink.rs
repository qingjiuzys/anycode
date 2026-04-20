//! REPL / `run` 共享输出目标（stdio vs 流式 TTY）。

use std::io::Write;
use std::sync::{Arc, Mutex};

use crate::repl::{ReplLineState, StreamReplRenderMsg};

/// REPL 输出目标：管道/工作流走真实终端；**Stream** 面向 `anycode repl` TTY：主缓冲下 echo 经 [`StreamReplRenderMsg`] 由 UI 线程 `insert_before` 进宿主 scrollback；备用屏时仅写 `transcript`。
///
/// **执行中**（`ReplLineState::executing_since`）：工具/模型流式输出仍写入 `transcript`，但**不再**逐行发
/// `ScrollbackChunk` — 宿主 scrollback 仅在**回合结束**后由 `tasks_repl` 一次性写入（避免 ratatui `insert_before`
/// 高频调用把视口反复顶进宿主历史导致叠行）。
pub(crate) enum ReplSink {
    Stdio,
    Stream {
        state: Arc<Mutex<ReplLineState>>,
        tail: String,
        render_tx: std::sync::mpsc::Sender<StreamReplRenderMsg>,
    },
}

impl ReplSink {
    pub(crate) fn line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        match self {
            ReplSink::Stdio => println!("{s}"),
            ReplSink::Stream {
                state, render_tx, ..
            } => {
                let st = state.lock().unwrap_or_else(|e| e.into_inner());
                let executing = st.executing_since.is_some();
                if st.stream_repl_host_scrollback && !executing {
                    let _ = render_tx.send(StreamReplRenderMsg::ScrollbackChunk(format!("{s}\n")));
                }
                let mut t = st.transcript.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(s);
                t.push('\n');
            }
        }
    }

    /// 与 `eprintln!` 对齐的 stderr 行。
    ///
    /// **`Stream` 不得写 stderr**：与 stdout 上的 Inline 视口交错后易叠字。应走 [`Self::line`] 写入 transcript（并 echo scrollback）。
    pub(crate) fn eprint_line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        match self {
            ReplSink::Stdio => eprintln!("{s}"),
            ReplSink::Stream { .. } => {}
        }
    }

    pub(crate) fn push_stdout_str(&mut self, s: &str) {
        match self {
            ReplSink::Stdio => {
                print!("{s}");
                let _ = std::io::stdout().flush();
            }
            ReplSink::Stream {
                state,
                tail,
                render_tx,
            } => {
                tail.push_str(s);
                loop {
                    let Some(i) = tail.find('\n') else {
                        break;
                    };
                    let line = tail[..i].to_string();
                    tail.drain(..i + 1);
                    let st = state.lock().unwrap_or_else(|e| e.into_inner());
                    let executing = st.executing_since.is_some();
                    if st.stream_repl_host_scrollback {
                        if !executing {
                            let _ = render_tx
                                .send(StreamReplRenderMsg::ScrollbackChunk(format!("{line}\n")));
                        }
                        // 执行中：宿主 scrollback 在回合结束后一次性写入；本路径不写 transcript。
                    } else if let Ok(mut t) = st.transcript.lock() {
                        t.push_str(&line);
                        t.push('\n');
                    }
                }
            }
        }
    }
}
