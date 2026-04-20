---
title: 会话通知
description: 通过 HTTP 或 shell 投递会话事件 JSON（可与 OpenClaw 类网关对接）。
read_when:
  - 你需要与 memory 管线钩子独立的出站通知。
---

# 会话通知（`config.json` → `notifications`）

在工具结果或 assistant 回合结束时，向 **HTTP URL** 和/或 **shell 命令（stdin）** 发送 **JSON**。与 **`memory.pipeline.hook_*`**（嵌入/记忆侧）**解耦**。

- **字段与安全：** 见 [配置与安全](./config-security)。
- **MCP stdio 生命周期（维护者）：** 仓库 [`docs/mcp-stdio-lifecycle.md`](https://github.com/qingjiuzys/anycode/blob/main/docs/mcp-stdio-lifecycle.md)

## 字段（摘要）

| 字段 | 含义 |
|------|------|
| `after_tool_result` | 每条工具结果后是否触发（需已配置投递方式）。 |
| `after_agent_turn` | 本轮 assistant 结束且无后续 tool_calls 时是否触发。 |
| `http_url` | 仅允许 **`http` / `https`** 的 POST，正文为 JSON。 |
| `http_timeout_ms` | HTTP 客户端超时。 |
| `http_headers` | 额外头；值中 **`${VAR}`** 从环境展开（未设置则为空串）。 |
| `shell_command` | Unix：`/bin/sh -c`；Windows：`cmd /C`；**stdin 写入 JSON**（UTF-8）。 |
| `shell_timeout_ms` | 子进程墙钟超时。 |
| `max_body_bytes` | 限制序列化后的 **`excerpt`**；合法范围为 **256–524288**。 |
| `tool_deny_prefixes` | 工具名以前缀匹配则跳过（如 `mcp__`）。 |

**`http_url`** / **`shell_command`** 若为空或仅空白，视为未配置该通道（与代码 `is_configured()` 一致）。

## JSON 载荷

每次投递含 **`schema_version`: 1** 与唯一 **`event_id`**（UUID 字符串），便于网关去重。

```json
{
  "schema_version": 1,
  "event_id": "6ba7b810-9dad-11d1-80b4-00c04fd430c8",
  "event": "tool_result",
  "session_id": "…",
  "task_id": "…",
  "turn": 2,
  "tool_name": "bash",
  "excerpt": "…",
  "excerpt_truncated": false,
  "timestamp": "2026-04-20T12:00:00.000Z",
  "working_directory": "/path/to/project"
}
```

## 与 `memory.pipeline` 钩子的区别

| | **`notifications`** | **`memory.pipeline.hook_*`** |
|--|--------------------|--------------------------------|
| 用途 | 出站集成（网关、脚本） | 记忆/嵌入管线 |
| 载荷 | 版本化 JSON（`schema_version`、`event_id` 等） | 管线专用 |
| 失败 | 仅日志；**不**阻断编排 | 依钩子实现而定 |

## OpenClaw 网关最小对接

将 **`http_url`** 指向网关（如 `https://gateway.example/hooks/anycode`）。可在 **`http_headers`** 中设置 **`Authorization: Bearer ${OPENCLAW_TOKEN}`**。服务端应 **POST 接收 JSON**、返回 **2xx**，并按需用 **`event_id`** 做幂等。

## 可观测性

**`tracing` `debug`**、target **`anycode_session_notify`**：记录 HTTP **host**、**event**、**excerpt_truncated**、**elapsed_ms**（不记录完整 excerpt 或密钥）。

English: [Session notifications](/guide/notifications).
