//! **阶段 2（备选）**：reedline 单行编辑 + 历史的最小 smoke，用于评估 IME 与终端组合。
//!
//! 与现有行式 REPL 的 **`InputState` 多行、斜杠补全、审批条** 对齐需要桥接层，工作量显著大于
//! `Viewport::Inline` 路径；默认栈已改为 ratatui Inline（见 `repl_stream_ratatui`）。
//!
//! 运行：`cargo run -p anycode --example reedline_spike`

use reedline::{DefaultPrompt, Reedline, Signal};

fn main() -> std::io::Result<()> {
    let mut editor = Reedline::create();
    let prompt = DefaultPrompt::default();
    loop {
        match editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => {
                println!("echo: {line}");
            }
            Ok(Signal::CtrlD) | Ok(Signal::CtrlC) => break,
            Err(e) => {
                eprintln!("reedline: {e}");
                break;
            }
        }
    }
    Ok(())
}
