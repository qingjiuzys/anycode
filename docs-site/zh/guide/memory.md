---
title: 记忆系统说明
description: anyCode 记忆与 OpenClaw 式 memory 扩展的对照备忘。
summary: 当前后端与 scope；后续可对标的改进清单。
read_when:
  - 对比 OpenClaw / Claude Code 的记忆行为。
---

# 记忆系统说明

## anyCode 现状

- **后端**：`memory.backend` 支持 `file` / `hybrid` / `noop`（见 [配置与安全](./config-security)）。
- **作用域**：项目 / 用户记忆经 `anycode_memory`，当前以关键词检索为主。
- **自动保存**：由 `memory.auto_save` 与任务成功后的 runtime 钩子控制。

## 与 OpenClaw 对标（研究 backlog）

OpenClaw 将记忆做成**扩展**，保留策略与召回路径独立。可对标项：

1. **写入时机**：仅在任务成功 vs 工具写入 vs 显式命令。
2. **检索**：关键词 vs 向量 / 混合；项目隔离保证。
3. **与压缩关系**：`/compact` 与会话自动压缩后记忆如何保留。

建议在 issue 里程碑跟踪，而非在 CLI 二进制内复制 OpenClaw 全部实现。

## 相关

- [架构](./architecture)  
- [配置与安全](./config-security)  
