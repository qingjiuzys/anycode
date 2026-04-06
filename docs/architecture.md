# anyCode 架构说明

面向维护者：分层、依赖方向与扩展点，避免「为抽象而抽象」。

## 分层与数据流

```text
anycode (CLI)          ← 组合根：读配置、组装 runtime、TUI/守护进程
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
3. **压缩前后行为**：实现 `CompactionHooks` 或通过 `AgentRuntime::new(..., Some(hooks))` 注入（默认 `DefaultCompactionHooks`）。
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
