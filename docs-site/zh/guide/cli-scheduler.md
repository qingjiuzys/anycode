---
title: 定时任务与调度器
description: CronCreate 落盘、anycode scheduler 执行、单实例锁与微信内嵌。
summary: orchestration.json、scheduler.lock、微信桥内嵌与独立 scheduler 二选一。
read_when:
  - 你需要类似 OpenClaw 的定时 agent 任务。
---

# 定时任务与内置调度器

## 能力说明

1. **工具 `CronCreate` / `CronDelete` / `CronList`**：把规则写入 **`~/.anycode/tasks/orchestration.json`**。表达式为 `cron` crate 语义（6 段：`秒 分 时 日 月 周`；传统 5 段会在前补 `0` 秒，见源码 **`crates/cli/src/scheduler.rs`**）。
2. **`anycode scheduler`**：常驻子命令，读同一 JSON，到点把每条任务的 **`command`** 当作**单次** agent 提示执行（与 `anycode run` 同类）。

**仅写 JSON 不会自动跑**：必须有一个调度循环在跑（见下）。

## 单实例锁 `scheduler.lock`

同机只应有一个调度循环，通过 **`~/.anycode/tasks/scheduler.lock`** 独占锁实现。

- 若已有一个 **`anycode scheduler`** 进程，再启动第二个会在日志中提示锁被占用并退出。
- **微信桥**可在 **`run_wechat_daemon`** 里 **内嵌** 一个调度循环（`tokio::spawn`），这样多数用户**不必**再单独起 `anycode scheduler`；若你仍单独起了一个，第二个会因锁而不起作用（符合预期）。

## 独立运行 `anycode scheduler`（可选）

适合与微信桥分离、或不在本机跑微信时。可用终端、**tmux**、**systemd user**、**macOS LaunchAgent** 等托管。

**systemd user 示例（Linux）** — 请改路径与工作目录：

```ini
[Unit]
Description=anyCode 内置 cron 调度器
After=network-online.target

[Service]
Type=simple
ExecStart=/usr/local/bin/anycode scheduler -C /path/to/workspace --reload-secs 30
Restart=on-failure

[Install]
WantedBy=default.target
```

**LaunchAgent 片段（macOS）**：

```xml
<key>ProgramArguments</key>
<array>
  <string>/usr/local/bin/anycode</string>
  <string>scheduler</string>
  <string>-C</string>
  <string>/path/to/workspace</string>
  <string>--reload-secs</string>
  <string>30</string>
</array>
```

同机不要故意起两个调度循环；第二个会因锁空转退出。

## 通道模式（微信 / Telegram / Discord）

**`workspace-assistant`** 已暴露 **`CronCreate` / `CronDelete` / `CronList`**，便于在对话里管理定时任务。向用户说明：**注册成功 ≠ 已到期执行**；真正按点跑需要 **内嵌调度器** 或 **单独的 `anycode scheduler`**，且 **同一时刻只有一个** 持锁进程在跑。

English: [Cron & scheduler](/guide/cli-scheduler).
