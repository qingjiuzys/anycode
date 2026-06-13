# 流式终端「丝滑」基线与回归清单

用于对比迭代前后观感；**不**追求与 Claude Code 像素级一致。

## Phase 0 — 可验收指标

| 指标 | 如何测 | 目标 |
|------|--------|------|
| 稳态刷新撕裂 | 大段输出/状态行变化时是否「横条切开」半帧 | 启用 CSI `?2026` 同步更新后应减轻（支持该模式的终端） |
| 首帧混排 | 主缓冲、`CLEAR_ON_START=0`、屏上原有 shell 输出 | 仍可能叠画；备用屏或清屏可规避 |
| 首帧 ANSI 体积 | `script -q /tmp/t.ty` 或重定向 `stdout` 录一段启动 | 留基线字节数供对比（非硬阈值） |
| 退出 scrollback | 备用屏退出后是否 dump、resume 提示是否可读 | 与 `NO_SCROLLBACK_DUMP` 一致 |
| PgUp/PgDn / 鼠标 | Workspace 滚动与审批菜单 | 与 `ANYCODE_TERM_MOUSE` 文档一致 |

## 技术事实（避免重复造轮）

**ratatui 0.24** 的 `Terminal` 维护双缓冲，每帧对 `previous_buffer.diff(current_buffer)`，**仅将变更 cell** 交给 `Backend::draw`。因此与 Ink 类似，**帧间已是增量写盘**；观感问题多来自 **首帧视口未接管**、**CSI 同步**、或 **resize/清屏** 路径，而非「每帧全格重写」。**默认 DEC 备用屏**（与 OpenClaw 类全屏终端 UI 一致）可避免整屏矩阵与主缓冲 scrollback 混排。CSI `?2026` 等由 crossterm / ratatui 在各自 `draw` 路径中处理（`terminal_guard` 仅保留**备用屏/内联**策略的环境解析）。

### 首帧与退出顺序（流式 REPL 摘要）

| 阶段 | 顺序 |
|------|------|
| 进入 | 按 [`stream_repl_use_alternate_screen`](../crates/cli/src/term/terminal_guard.rs) 选备用屏或主缓冲 Inline；`Terminal::new` + ratatui 主循环 |
| 退出 | `shutdown_stream_terminal` / 备用屏与 scrollback dump 策略见 `stream_term`；会话 JSON 落盘在 `~/.anycode/sessions` |

**Resize 防抖**：`resize_debounce` 模块在尺寸**连续快速变化**时跳过部分帧；**第一次**观察到终端尺寸时**不会**跳过，避免极短启动间隔下漏掉首帧。

## 环境变量（摘录）

| 变量 | 含义 |
|------|------|
| `ANYCODE_TERM_ALT_SCREEN` | 与流式/通用备用屏策略相关（见 `terminal_guard`）。要关：须 **`export ANYCODE_TERM_ALT_SCREEN=0`** 或同进程一行设置；或 `config.json` 中 **`terminal.alternateScreen`: `false`**；`tmux -CC` 等仍有一致启发式。 |
| `ANYCODE_TERM_CLEAR_ON_START` | 仅**主缓冲**路径：首帧是否 `Clear(All)`（保留解析，当前全屏主循环已移除） |
| `ANYCODE_TERM_SYNC_DRAW` | `1`（默认，解析保留）：`?2026` 相关策略 |
| `ANYCODE_TERM_MOUSE` | 文档与帮助字符串（鼠标记策略随入口变化） |
| `ANYCODE_TERM_REPL_INLINE_LEGACY` / `ANYCODE_TERM_REPL_ALT_SCREEN` | 流式 REPL 专用：内联/备用屏见 `stream_repl_use_alternate_screen` |

## Phase 3 — 终端矩阵（人工）

在以下环境各跑一遍「启动 → 发一条 → 退出」，勾选无异常。

- [ ] macOS Terminal.app
- [ ] iTerm2
- [ ] tmux（普通）
- [ ] tmux `-CC`（若适用）
- [ ] Alacritty / WezTerm 等 GPU 终端

关注：闪屏、残影、`resize`、鼠标滚轮、备用屏退出。

## Transcript 虚拟滚动（backlog）

此前预研的 `virtual_scroll` 未接入主绘制路径，已从默认编译路径移除，避免长期保留未使用代码。后续若重启该方向，建议先以 RFC 形式明确目标负载（会话长度、滚动频率、延迟预算），再按本文件 Phase 0 指标复测（大段 assistant 输出、快速 PgUp/PgDn）。

## 录制参考命令（可选）

```bash
# 录 ANSI 流（示例）
script -q /tmp/anycode-term.script env ANYCODE_TERM_SYNC_DRAW=1 anycode
# 退出后查看体积
wc -c /tmp/anycode-term.script
```
