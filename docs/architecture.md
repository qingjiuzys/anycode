# anyCode 架构说明

面向维护者：分层、依赖方向与扩展点，避免「为抽象而抽象」。

**文档站**（中英、与发布流程一致）：仓库根目录 [`docs-site/guide/architecture.md`](../docs-site/guide/architecture.md) 构建为在线「Architecture」页；扩展操作清单见 [`docs-site/guide/contributing-extensions.md`](../docs-site/guide/contributing-extensions.md)。**ADR**（编排边界等决策）在 [`docs/adr/`](adr/)，不参与 VitePress 构建。

## 分层与数据流

```text
anycode (CLI)          ← 组合根：读配置、组装 runtime、TUI / run / 通道桥
    ↓
anycode-agent          ← AgentRuntime：一轮/多轮 LLM + 工具、压缩、落盘
    ↓
anycode-core           ← 领域类型 + 稳定 trait（Tool / LLMClient / MemoryStore / …）
    ↑
anycode-tools          ← 工具实现 + build_registry（注册表）
anycode-llm            ← LLM 适配
anycode-security       ← 审批与策略
anycode-memory         ← 记忆后端
```

**依赖规则**

- `core` 不依赖 agent / cli / tools。
- `agent` 依赖 `core`、`tools`、`security`；编排多轮循环，不实现具体工具。
- `cli` 依赖上述 crate，在 `bootstrap` 中构造 `AgentRuntime` 与工具注册表。

## 扩展点（优先使用顺序）

1. **新工具**：在 `anycode-tools` 的 `registry.rs` 注册实例，并维护 `catalog` 常量；详见该文件顶部 checklist。
2. **新 LLM 提供商**：在 `anycode-llm` 实现 `LLMClient`。
3. **压缩前后行为**：见 `compact` 模块与 `CompactionHooks`（当前 `AgentRuntime::new` 使用默认钩子；构造参数分组为 `RuntimeCoreDeps` / `RuntimeMemoryOptions` / `RuntimeToolPolicy`）。
4. **新 Agent 类型**：实现 `Agent` trait 并 `register_agent`，或扩展内置 `agents.rs`。

仅在确有第二种实现或需打破依赖环时，再新增 trait。

## Crate 要点

| Crate | 职责 |
|--------|------|
| `core` | `Message` / `Task`、错误、`Tool`·`LLMClient`·`MemoryStore` 等；源码按子模块拆分（`message`、`task`、`traits` 等），`lib.rs` 仅聚合导出。 |
| `agent` | `AgentRuntime` 门面；`runtime/mod.rs` 保留构造与工具执行门控，`runtime/session.rs` 承载 `execute_task` / `execute_turn_from_messages` / `compact_session_messages`。 |
| `tools` | 工具实现与 `ToolRegistryDeps` 注入。 |
| `channels` | 多通道类型与预留实现；主 CLI 路径可不依赖。 |

## 设计原则（Code review 备忘）

- 新人应能在约 30 分钟内说清：请求从 CLI 进入 `AgentRuntime` 后，何处调用 `LLMClient::chat` 与 `Tool::execute`。
- 子模块拆分优先于新抽象；避免通用 `PipelineStage<T>` 类框架，除非多产品线硬需求。

## CLI：流式 REPL（Inline）与全屏 TUI 的会话一致性

两者都通过 **`AgentRuntime::execute_turn_from_messages`** 跑多轮工具循环，并共用 **`~/.anycode/tui-sessions/`** 下的 JSON 快照（`ReplLineSession` 与 TUI 同一持久化格式）。

| 能力 | 流式 REPL（默认 TTY） | 全屏 TUI |
|------|----------------------|----------|
| 会话 id / 恢复 | `ReplLineSession::session_file_id`；`anycode repl --resume <uuid>` 与 TUI `--resume` 读同一目录 | `session_uuid`；退出提示与 `--resume` 一致 |
| `/session` | `list` / 无参 cwd 优先 / 显式 uuid → `load_repl_session_choice` + `apply_snapshot` | 同上逻辑，经 `TuiLoopCtl::ResumeSession` 在主循环里应用快照 |
| `/clear` | `repl_clear_session` → `rebuild_for_agent`，并清空流式 UI 状态（滚动、HUD 摘要、**last_turn_token_usage**、**stream_exit_dump_anchor**） | `reset_transcript_state` + 重建 messages；**last_max_input_tokens** / **last_turn_usage** 归零 |
| 持久化 | 回合结束 `spawn_persist_tui_session` | 同上 |
| Token 用量展示 | 回合结束 HUD 与 **`/context`** 使用 `TurnTokenUsage`（与 agent 返回的 **`TurnOutput.usage`** 对齐） | 脚标 **`last_output_tokens`** + **`/context`** 使用同一套聚合字段 |

退出 Inline 视口时，默认把 **完整** transcript 再打一份到 shell；**`ANYCODE_STREAM_EXIT_SCROLLBACK_DUMP=0`** 关闭；**`=anchor`** 仅打印自上一轮自然语言轮起的内容（与异步侧 `turn_transcript_anchor` 同步到 **`ReplLineState::stream_exit_dump_anchor`**）。

迭代任务与决策状态见 **[`docs/roadmap.md`](roadmap.md)**（SSOT）；MVP 与工具矩阵见文档站 [Roadmap](../docs-site/guide/roadmap.md)。
