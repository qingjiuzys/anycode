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

**Agent** / **Task** 出现在工具列表中时，表示嵌套子 Agent；与 Claude Code **`Agent`** 工具对应的 JSON 字段（**`cwd`**、**`model`**、**`isolation`**、**`run_in_background`** 等）见 [路线图](./roadmap) **P5** 段落。

## `test-security`

对给定工具名与 JSON 输入走一遍 **`SecurityLayer::check_tool_call`**：

```bash
./target/release/anycode test-security --tool Bash --input '{"command":"ls"}'
```

子命令名以 **`anycode --help`** 为准。

## `LSP` 工具

使用 **`--features tools-lsp`** 时，**`LSP`** 工具通过 stdio 子进程转发 JSON-RPC。在 `config.json` 中配置 **`lsp`**（见 [配置与安全](./config-security) 的 **LSP** 小节），或设置 **`ANYCODE_LSP_COMMAND`**。

## 相关

- [路线图](./roadmap) — **P5**：**Agent** / **Task** 与 Claude 字段对齐说明  
- [Agent skills](./skills) — **`anycode skills`**、**`SKILL.md`** 发现  
- [配置与安全](./config-security)  
- [架构](./architecture)  

English: [Discovery & test-security](/guide/cli-diagnostics).
