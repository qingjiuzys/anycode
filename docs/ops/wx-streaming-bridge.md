# 微信桥：流式与编排边界（设计备忘）

## 编排路径

- **微信 daemon**（`crates/cli/src/wx/bridge.rs`）当前对每条用户消息构建一轮 **`Task`**，调用 **`AgentRuntime::execute_task`**（单任务、多轮工具循环内聚在 runtime 内）。
- **流式 REPL / TUI** 等可走 **`execute_turn_from_messages`**，消息列表由调用方维护；微信会话状态里虽有 `chat_history`，但桥接层仍用 **`execute_task`**，与「逐条 Message 追加」的 REPL 模式不等价。

## 消息分片与速率

- 助手**最终回复**经 `split_message(..., CHUNK_MAX)`（默认 2048 字符）分片，经 iLink **`send_text`** 顺序发送。
- **工具进度**：不向微信推送 `🔧 tool` / `✓ tool` 等中间行；用户仅收到助手最终回复（及权限审批、定时提醒等显式消息）。
- **可选后续（未实现）**：若需可配置的轻量进度（节流后的单行状态），可复用 `TaskContext::channel_progress_tx` 并在桥侧过滤；**与 `execute_turn_from_messages` 全量流式对齐**仍属大项，见上文「编排路径」。

## 推理与展示策略

- 最终回复经 **`strip_llm_reasoning_for_display`** 与 `sanitize_wechat_reply_output` 去掉 `<thought>` 等及常见废话前缀。
## 取消语义

- **新消息打断**或处理中 **`/clear`**：对当前回合的 **`nested_cancel`**（`Arc<AtomicBool>`）置位，并 **`JoinHandle::abort`** 运行中任务，使 LLM/工具循环在协作边界尽快结束。
- `wx_turn_cancel` 在桥进程内指向**当前回合**的 flag；回合结束或 `execute_task` 返回后清空，避免误伤下一轮。

## 相关文件

- `crates/cli/src/wx/bridge.rs`
- `crates/core/src/task.rs`（`nested_cancel`；`channel_progress_tx` 保留供后续可选进度）
- `crates/agent/src/runtime/mod.rs`（工具边界）
