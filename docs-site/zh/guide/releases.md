---
title: 版本与特性开关
description: 版本号、GitHub Releases、以及 anycode enable/disable 实验能力。
summary: 更新发布渠道；用统一 CLI 入口切换运行时 feature。
read_when:
  - 发布或安装 anyCode 构建。
  - 需要 enable/disable 管理实验功能。
---

# 版本与特性开关

## 0.2.0（工作区）

- **模型**：Z.ai / 智谱 GLM 与 OpenClaw `model-definitions` 对齐；`plan` 的 `coding_cn` / `general_cn` 对应 `open.bigmodel.cn`；Google Gemini 目录；`anycode model` 路由向导用 OpenClaw 风格选厂商与 z.ai 端点。
- **通道**：`telegram-set-token`、`discord-set-token`；`anycode_channels::hub` 说明统一 `ChannelMessage` → `build_channel_task`；微信桥不再挂交互式工具审批回调。
- **LLM**：Anthropic 非流式 `chat` 对 429/5xx 与 `Retry-After` 重试（与 z.ai 客户端策略一致）。
- **Skills**：可选 `skills.registry_url` 合并扫描根、`skills.agent_allowlists` 按 agent 裁剪提示中的技能列表。
- **Agent**：嵌套 **`run_in_background`** 协作式取消（含进行中 LLM/流式；对嵌套任务 id 发 **`TaskStop`**）。
- **会话（全屏 TUI 与 TTY 流式 REPL）**：主路径 **`execute_turn_from_messages`** 上，回合进行中 **Ctrl+C** 触发同一套协作取消标志（全屏 TUI：首击取消回合，空闲时仍为连按退出；TTY **`anycode repl`**：进行中时 Ctrl+C 取消回合，不再把空行 Ctrl+C 当成直接退出）。
- **MCP / LSP**：MCP stdio **`ANYCODE_MCP_READ_TIMEOUT_SECS`**（JSON-RPC 按行读）、可选 **`ANYCODE_MCP_CALL_TIMEOUT_SECS`**（整次 **`tools/call`**）；超时/EOF 与子进程提示、**`McpStdioSession::stdio_child_is_running`**；**`config.json` `lsp`**；CI **`tools-lsp`** / **`tools-mcp`** 测试。

## 版本与发布

- **版本号**：工作区根目录 `Cargo.toml` 的 `version`。
- **GitHub Releases**：对常用平台打 tag 并附带 `anycode` 二进制（非 `cargo install` 场景）。
- **文档站**（`docs-site/` VitePress）：GitHub Pages 部署时设置 `VITEPRESS_BASE=/仓库名/`。

## 运行时特性（enable / disable）{#runtime-feature-flags}

```bash
anycode enable skills
anycode disable workflows
anycode status
```

名称与 `anycode_core::FeatureFlag` 一致：

| 能力 | enable / disable 参数 |
|------|------------------------|
| CLI skills 扫描 | `skills` |
| 工作流相关 | `workflows` 或 `workflow` |
| 目标模式配套 | `goal-mode` 或 `goal` |
| 通道模式配套 | `channel-mode` 或 `channel` |
| 实验审批 | `approval-v2` 或 `approval` |
| 上下文压缩配套 | `context-compression` 或 `compact` |
| 工作区 profile | `workspace-profiles` 或 `workspace` |

## 相关

- [总览](./cli)  
- [路由](./routing)  
