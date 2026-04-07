---
title: 排错
description: anyCode 常见问题：无 TTY、微信扫码、MCP 与文档链接。
summary: 非交互环境、桥接登录与 MCP/OAuth 限制的快速处理。
read_when:
  - setup / wechat / config 失败。
  - 在 CI 或无图形环境运行。
---

# 排错

## 无 TTY / SSH / CI

- **`anycode config`**、**`setup`** 在非 TTY 下可能无法交互，请在本机**真实终端**执行，或事先写好 **`~/.anycode/config.json`**。
- **微信扫码**需要能弹出浏览器或展示二维码的环境；可在有图形界面的机器上再执行 **`anycode channel wechat`**。
- **`run` / `repl`** 在 `require_approval=true` 时需要连着终端用 stdin 确认；**`--ignore-approval`** 仅在你清楚风险时使用。

## 微信桥

- 若向导提示运行 **`anycode channel wechat`**，请在桥接能完成扫码的环境执行。
- 微信任务默认工作目录常为 **`~/.anycode/workspace`**；可在微信内用 **`/cwd`** 切到项目（见 [微信与 setup](./wechat)）。

## MCP 与 OAuth

- 需 **`cargo build` 带 `tools-mcp`**，并检查 **`ANYCODE_MCP_COMMAND`** / **`ANYCODE_MCP_SERVERS`** 与 **`~/.anycode/tasks/<id>/output.log`**。
- **OAuth / `McpAuth`** 可能依赖 MCP 服务器的 stdio 交互，无头环境可能无法完成；优先使用可配置 token 的服务器。
- 若模型侧看不到 MCP 工具，检查 **`security.mcp_tool_deny_patterns`**（见 [配置与安全](./config-security)）。

## z.ai 首轮不出 tool_calls

- 可试 **`ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN=1`** 或配置里 **`zai_tool_choice_first_turn: true`**（见 [run / REPL / TUI](./cli-sessions)）。

## 文档里的「死链」

- 本站对仓库外 **`crates/`** 路径设置了 **`ignoreDeadLinks`**，便于在 GitHub 上点进源码；**`vitepress build`** 若仍报站内死链，请修正路径或在 **`.vitepress/config.ts`** 增加例外。

## 仍然无法解决

- 提 Issue 时请带 **系统**、**版本信息**、脱敏后的 **`config.json`**。  
- 本地测试见 [开发与贡献](./development)。
