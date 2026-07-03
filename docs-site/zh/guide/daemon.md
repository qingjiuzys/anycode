---
title: 无头守护进程
description: anycode-daemon 在无桌面环境下运行通道桥接与 cron 调度。
---

# 无头守护进程（`anycode-daemon`）

独立二进制 **`anycode-daemon`** 承接已退役终端 `anycode` CLI 中的常驻服务：

| 子命令 | 用途 |
|--------|------|
| `scheduler` | Cron / 自动化触发循环 |
| `wechat-bridge` | 个人微信 iLink 桥接 |
| `telegram-bridge` | Telegram 机器人桥接 |
| `discord-bridge` | Discord 机器人桥接 |

安装见 [安装](./install)（Linux/Windows 预编译包或从 `crates/channel-bridge` 源码 `cargo install`）。

## 示例

```bash
anycode-daemon scheduler
anycode-daemon wechat-bridge
anycode-daemon telegram-bridge
anycode-daemon discord-bridge
```

配置位于 `~/.anycode/config.json`（与 Workbench 相同）。首次模型配置请在 Workbench **`/setup`** 完成 — 桌面应用或内嵌 dashboard。

## 桌面 vs 守护进程

| 场景 | 选择 |
|------|------|
| macOS 日常使用 | **anyCode.app**（工作台 + 原生 STT/OCR） |
| Linux 服务器 / NAS | **`anycode-daemon`** 跑通道与调度 |
| 仅定时任务 | `anycode-daemon scheduler`（或保持桌面应用运行） |

旧版 HTTP `anycode daemon`（POST `/v1/tasks`）已移除 — 见 [ADR 003](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/003-http-daemon-deprecated.md)。

## 相关

- [定时提醒](./cli-scheduler)
- [微信与配置](./wechat)
- [安装](./install)

English: [Headless daemon](/guide/daemon).
