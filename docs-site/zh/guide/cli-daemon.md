---
title: HTTP 守护进程（已移除）
description: anycode HTTP daemon 子命令已删除；请用 run、REPL/TUI 或通道桥。
summary: 历史说明与 ADR 003；仓库内不再提供 localhost POST /v1/tasks。
read_when:
  - 你打开了旧的 `anycode daemon` 或 POST /v1/tasks 文档链接。
---

# HTTP 守护进程（已移除）

**`anycode daemon`** HTTP 服务（**`GET /health`**、**`POST /v1/tasks`**）已 **不作为产品能力维护**。CLI 会将 **`daemon`** 子命令视为 **已移除**（与其它废弃子命令一样拒绝），原先的 `daemon_http` 模块已从默认路径删除。

**请改用**

- 脚本/CI：**`anycode run`**  
- 交互：**`anycode repl`** / **`anycode tui`**  
- 常驻或定时：**`anycode channel …`**、**`anycode scheduler`**  
- 若需要自建 HTTP：在外部服务里 **调用 CLI**，而非依赖仓库内嵌 HTTP daemon

**决策记录：** [ADR 003](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/003-http-daemon-deprecated.md)。

**维护者 backlog：** [`docs/roadmap.md`](https://github.com/qingjiuzys/anycode/blob/main/docs/roadmap.md)。

## 相关

- [CLI 总览](./cli) — 当前子命令  
- [架构](./architecture)  
- [路线图](./roadmap) — MVP 与工具矩阵（不含 daemon）

English: [HTTP daemon (removed)](/guide/cli-daemon).
