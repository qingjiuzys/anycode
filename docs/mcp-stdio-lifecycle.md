# MCP stdio 会话生命周期（实现备忘）

面向维护者：描述 `crates/tools/src/mcp_session.rs` 中长驻 stdio 会话从启动到失败时的行为，与排错文案一致。

## 启动

1. **`McpStdioSession::connect(command_shell, server_slug)`** 用 `sh -c` 拉起子进程，`stdin`/`stdout` 管道。
2. 按 MCP 顺序写入 JSON-RPC 行（每行一条消息，含换行）：
   - `initialize`（id=1）
   - `notifications/initialized`（notification）
   - `tools/list`（id=2）
3. **`read_until_id`**：在超时内从 stdout **按行**读，直到 `id` 匹配；空行跳过；非 JSON 行跳过。

## 运行期

- **`rpc`**：在 `Mutex` 内串行写 stdin、读响应，避免交错。
- **`tools/call`**：可选整段墙钟超时 `ANYCODE_MCP_CALL_TIMEOUT_SECS`；单行读仍受 `ANYCODE_MCP_READ_TIMEOUT_SECS` 约束。
- **`stdio_child_is_running`**：`try_wait` 判断子进程是否已退出。

## 失败形态（用户可见错误）

| 情况 | 典型错误信息要点 |
|------|------------------|
| 子进程立即退出 / stdout 提前 EOF | `unexpected end of stdout`、`child exited: …` |
| 单行读超时 | `MCP read timed out`、`id=…`、`ANYCODE_MCP_READ_TIMEOUT_SECS` |
| 整次 `tools/call` 超时 | 墙钟超时文案（与 `ANYCODE_MCP_CALL_TIMEOUT_SECS` 相关） |
| 1024 行内仍无匹配 id | `no JSON-RPC response with matching id` |

## 重连策略

**当前实现不自动重连。** 会话与 `Child` 绑定；子进程退出或长期失败后，需由上层（新一次工具注册/新进程）重新 `connect`。文档站排错中与此保持一致；若未来引入自动重连，需单独定义触发条件、退避与错误面。

## 相关代码

- `read_until_id`、`McpStdioSession::connect`、`call_tool_named`
- 超时：`crates/tools/src/mcp_read_timeout.rs`
