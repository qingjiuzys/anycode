---
title: 版本与特性开关
description: 版本号、GitHub Releases、以及 anycode enable/disable 实验能力。
summary: 更新发布渠道；用统一 CLI 入口切换运行时 feature。
read_when:
  - 发布或安装 anyCode 构建。
  - 需要 enable/disable 管理实验功能。
---

# 版本与特性开关

## 0.3.2（工作区）

- **开放验证**：Discover→Search→Run（`verify-discover` skill + 空口完成 evidence nudge + `verify_recipe` 记忆）。
- **启动短路移除**：主机「站点已在运行 :8080」不再劫持 docker/真实启动。
- **套餐**：Free 新用户 2000 万 tokens；Cloud 5h 窗口内 1000 次；Pro ¥599/月窗口内 10000 次（Pro 模型暂未开放）。
- **门户案例**：https://anycode.work/cases/… 可「打开演示」在线翻页/预览。
- **首页斜杠**：Workbench 首页支持 `/拷问`、`/目标`。
- **CI**：`mime_to_ext` 跨平台；fmt 漂移修复。

## 0.3.1（工作区）

- **交付物卡片**：表格（CSV/XLSX/workbook.json）缩略预览、Office/PDF `preview.html` 侧车、PPT 幻灯片网格、正文大表格卡、Mermaid 流程图块。
- **Viewer 收敛**：对话卡片与工作台预览共用 `selectDeliverableViewer` 路由。
- **Skill emit**：`anycode-ppt` 及 office 系列 starter 统一 `ANYCODE_ARTIFACT` + sidecar。

## 0.3.0（工作区）

- **拷问模式 / Grill Me**：先对齐再动手（`AskUserQuestion` 单问）。
- **团队交接**：局域网 mDNS（ADR 015）+ 云端 A2A 流式中继（ADR 016，不经 OSS）。
- **发布打包**：macOS 仅发 **`anyCode_<version>_aarch64.dmg`**（CLI 已内置）；Linux/Windows CLI 请用 `cargo install` 或源码。

## 0.2.2（工作区）

- **发布打包**：macOS 仅发 **`anyCode_<version>_aarch64.dmg`**（CLI 已内置）；tag 不再自动附带 Linux/Windows CLI tar/zip，请用 `cargo install` 或源码安装。
- **桌面 CI**：无 Apple 开发者证书时使用 ad-hoc 签名；tag 触发时 desktop release 仅跑 macOS。

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
- **GitHub Releases**：打 tag 仅附带 **macOS Tauri `.dmg`** — CLI 内置在 `anyCode.app` 的 `Contents/Resources/resources/bin/anycode`（见 [数字工作台 — 桌面应用](./dashboard#桌面应用-macos)）。**Linux / Windows** 不再发布独立 CLI 包；请用 `cargo install` 或源码（`scripts/install.sh --method source`）。
- **文档**（`docs/user/`）：通过 account-portal 构建发布到 **https://anycode.work/docs/**。

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
