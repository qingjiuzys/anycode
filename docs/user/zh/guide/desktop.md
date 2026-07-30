---
title: 桌面应用（macOS）
description: anyCode.app 内置数字工作台、原生语音与设备端 OCR。
---

# 桌面应用（macOS）

**anyCode.app** 是 macOS 上的推荐使用方式。

## 与浏览器版有何不同

| 能力 | 桌面应用 | 仅浏览器访问 43180 |
|------|----------|-------------------|
| 完整工作台 UI | ✅ 应用窗口内 | ❌ 仅 API |
| Apple 语音输入 | ✅ | ❌ |
| 设备端 OCR（Vision） | ✅ | ❌ |
| 内置浏览器（Agent 自动化） | ✅ | 视构建而定 |

## 安装

1. [GitHub Releases](https://github.com/qingjiuzys/anycode/releases) → **`anyCode_<version>_aarch64.dmg`**
2. 拖入「应用程序」
3. 打开 **anyCode**

![工作台在应用内打开](/docs/assets/screenshots/home.png)
*图：桌面应用内的工作台*

## 首次配置

应用首次启动会进入 **设置向导**（`/setup`）。配置模型 API Key 与记忆选项即可开始对话。

详见 [快速开始](./getting-started)。

## 保持运行（定时任务）

若使用 [定时提醒](./cli-scheduler)，请保持 anyCode 在后台运行，或使用服务器上的 **`anycode-daemon`**。

## 相关文档

- [打开工作台](./dashboard)
- [工作台导览](./workbench)
- [常见问题](./troubleshooting)

English: [Desktop app (macOS)](/docs/guide/desktop).
