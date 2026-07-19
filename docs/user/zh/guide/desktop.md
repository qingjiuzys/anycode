---
title: 桌面应用（macOS）
description: anyCode.app 内置数字工作台、原生语音与设备端 OCR。
---

# 桌面应用（macOS）

**anyCode.app** 是 macOS 上的推荐方式。Workbench 在应用窗口内打开（不是单独浏览器标签页），并提供浏览器版没有的原生能力：

- **Apple Speech** — 语音输入，无需下载 Whisper
- **Apple Vision OCR** — 设备端图片文字识别

## 安装

1. 从 [Releases](https://github.com/qingjiuzys/anycode/releases) 下载 **`anyCode_<version>_aarch64.dmg`**。
2. 打开镜像，将 **anyCode** 拖入「应用程序」。
3. 启动 **anyCode** — Workbench 在应用窗口内打开。

::: tip
macOS 请使用 **anyCode.app** 窗口操作 Workbench。桌面版运行时 `http://127.0.0.1:43180` 仅提供 API（Safari/Chrome 无法打开完整 UI）。无头/Linux 安装仍通过浏览器访问该地址。
:::

## 首次配置

在 Workbench 打开 **设置** 或访问 **`/setup`**，配置模型（BYOK）、记忆与可选通道。

## 无头通道与定时任务

日常使用可保持桌面应用运行。若在服务器上跑微信/Telegram/Discord 或独立调度进程，请安装 **`anycode-daemon`** — 见 [无头守护进程](./daemon)。

## 相关

- [打开工作台](./dashboard)
- [工作台导览](./workbench)
- [快速开始](./getting-started)

English: [Desktop app (macOS)](/guide/desktop).
