---
title: 快速开始
description: 安装 anyCode，完成 Workbench 配置，并在几分钟内跑通第一条任务。
summary: 面向非技术用户的最短路径，包含失败时下一步动作。
read_when:
  - 第一次使用 anyCode，想尽快跑通。
---

# 快速开始

适合第一次使用 anyCode 的用户。

完成本页后，你会得到：

- anyCode 已安装（桌面应用或 `anycode-daemon`）
- 已通过 Workbench **`/setup`** 配置模型
- 一条验证对话成功返回

## 五分钟路径

1. **安装** — 见 [安装](./install)：macOS 推荐 **anyCode.app**，Linux/Windows 安装 **`anycode-daemon`**。  
2. **打开工作台** — 启动桌面应用，或访问 `http://127.0.0.1:43180`。  
3. **完成 `/setup`** — 选择模型、记忆 / 向量（见 [记忆](./memory)），可选通道。  
4. **验证** — 在工作台会话中发送一条短消息。

## 环境要求

- **预编译安装**：不需要 Rust。
- **源码构建**：需要 Rust + Cargo（`cargo build --release -p anycode-desktop-desktop-channel-bridge` 或 desktop crate）。
- **微信扫码**：在可打开浏览器/GUI 的机器上运行 `anycode-daemon wechat-bridge`。

## 首次配置（Workbench）

打开 **`http://127.0.0.1:43180/setup`**（或应用内 **设置**），按向导完成：

1. 模型 / provider（BYOK）
2. 记忆与可选向量
3. 可选通道说明

配置保存在 `~/.anycode/config.json`，无终端 `Workbench /setup` 命令。

## 验证

在工作台首页或项目会话中发送：

> 请只回复：OK

预期：助手回复 `OK`。

## 下一步体验路线

| 目标 | 操作 | 文档 |
|------|------|------|
| **macOS 日常使用** | **anyCode.app** | [桌面应用](./desktop) |
| **工作台** | 项目、会话、资产、审批 | [工作台导览](./workbench) |
| **个人微信** | `anycode-daemon wechat-bridge` | [微信与配置](./wechat) |
| **定时任务** | 工作台 **Automations** + `anycode-daemon scheduler` | [定时提醒](./cli-scheduler) |
| **无头服务器** | `anycode-daemon` 通道与调度 | [无头守护进程](./daemon) |
| **模型 / BYOK** | 工作台 **设置** | [模型与端点](./models) |

## 失败时下一步

- 工作台打不开 → 确认桌面应用已启动，或端口 **43180** 可访问
- `anycode-daemon: command not found` → 见 [安装](./install) PATH 说明
- 微信扫码失败 → 在 GUI 机器运行 `anycode-daemon wechat-bridge`

## 界面语言

在工作台 **设置** 中切换，或：

```bash
export ANYCODE_LANG=zh
```

## 下一步

- [安装](./install) · [模型](./models) · [工作台](./workbench) · [微信](./wechat) · [排错](./troubleshooting)

English: [Getting started](/guide/getting-started).
