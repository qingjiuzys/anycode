# claude-code-rust 与 anyCode Stream REPL 对照备忘

本仓库注释中的「claude-code-rust」主要指社区实现 **[srothgan/claude-code-rust](https://github.com/srothgan/claude-code-rust)**（Apache-2.0），而非 Anthropic 私有仓库。本地对照可执行：

```bash
git clone --depth 1 https://github.com/srothgan/claude-code-rust.git /tmp/claude-code-rust
```

## 模块对应

| anyCode | claude-code-rust（典型路径） |
|---------|-------------------------------|
| [crates/cli/src/repl/stream_viewport.rs](../../crates/cli/src/repl/stream_viewport.rs) | `src/ui/chat.rs` — 折行、滚动条几何、平滑滚动 |
| [crates/cli/src/repl/dock_render.rs](../../crates/cli/src/repl/dock_render.rs) | `src/ui/` 下 prompt / status 组合（结构不同，需自行搜 `Paragraph`、`HUD`） |
| [crates/cli/src/repl/stream_ratatui.rs](../../crates/cli/src/repl/stream_ratatui.rs) | 全应用 `App` + 单事件循环；anyCode 拆为 **Tokio 回合循环** + **std 线程 UI** |
| [crates/cli/src/repl/exec_parity.rs](../../crates/cli/src/repl/exec_parity.rs) | 回合生命周期在 `App` / bridge 内统一；可搜 `SpinnerState`、`AppStatus` |

## 行为差异（速览）

| 主题 | claude-code-rust | anyCode Stream REPL |
|------|------------------|---------------------|
| 架构 | 单进程 ratatui + App 状态机 | `Viewport::Inline` + Tokio 回合循环；执行中增量经队列 + UI 线程 **`insert_before`**（宿主 scrollback） |
| 主缓冲滚动 | 项目自述强调 native scrollback 等 | 执行中 `build_stream_turn_plain` → **`StreamReplRenderMsg`** → UI `insert_before`；Inline 主区同步 `transcript` 摘要 |
| 鼠标 | 随其 ratatui 策略 | Stream REPL **不**启用鼠标捕获，滚轮交给终端 scrollback |

## 维护建议

对齐 HUD/执行态时，优先对照其 **`AppStatus` / `SpinnerState`** 与 anyCode 的 **`ReplLineState::executing_since`**（[`line_state.rs`](../../crates/cli/src/repl/line_state.rs)）+ **[`sync_repl_dock_status`](../../crates/cli/src/tasks/tasks_repl.rs)** + **[`dock_render`](../../crates/cli/src/repl/dock_render.rs)**（Prompt HUD 与耗时秒数）；长文滚动对齐 `chat.rs` 中 scrollbar 与 `SCROLLBAR_*` 常量与 [stream_viewport.rs](../../crates/cli/src/repl/stream_viewport.rs) 中 `SCROLL_EASE` 等命名。
