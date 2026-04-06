---
title: 守护进程（HTTP）
description: anycode daemon — 与 run 共用运行时，健康检查与任务接口。
summary: 绑定地址、POST /v1/tasks 与可选令牌。
read_when:
  - 需要通过本机 HTTP 触发任务。
---

# 守护进程（HTTP）

与 **`run`** **共用同一套** `initialize_runtime`（LLM、工具注册、`SecurityLayer`、沙箱等）。

```bash
./target/release/anycode daemon --bind 127.0.0.1:8080
```

- **`GET /health`**：返回纯文本 **`ok`**。  
- **`POST /v1/tasks`**：`Content-Type: application/json`：

```json
{
  "agent": "general-purpose",
  "prompt": "你的任务描述",
  "working_directory": null
}
```

**`working_directory`** 可省略或 `null`，表示进程当前目录（会 canonicalize）。

若设置 **`ANYCODE_DAEMON_TOKEN`**，则 **`POST /v1/tasks`** 需携带 **`Authorization: Bearer <token>`** 或 **`X-Anycode-Token: <token>`**。**`/health`** 不受令牌保护。建议仅绑定本机。

## 相关

- [架构](./architecture)  
- [排错](./troubleshooting)  

English: [Daemon](/guide/cli-daemon).
