---
title: 发现与 test-security
description: list-agents、list-tools 与 test-security 本地策略调试。
summary: 自省子命令与 SecurityLayer 单次校验。
read_when:
  - 要在终端列出 agent / 工具。
  - 要排查工具调用被拒绝的原因。
---

# 发现与 test-security

## 列出 Agents / 工具

```bash
./target/release/anycode list-agents
./target/release/anycode list-tools
```

## `test-security`

对给定工具名与 JSON 输入走一遍 **`SecurityLayer::check_tool_call`**：

```bash
./target/release/anycode test-security --tool Bash --input '{"command":"ls"}'
```

子命令名以 **`anycode --help`** 为准。

## 相关

- [Agent skills](./skills) — **`anycode skills`**、**`SKILL.md`** 发现  
- [配置与安全](./config-security)  
- [架构](./architecture)  

English: [Discovery & test-security](/guide/cli-diagnostics).
