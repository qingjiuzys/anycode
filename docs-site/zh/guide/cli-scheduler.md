---
title: 定时任务与调度器
description: CronCreate 落盘、anycode scheduler 执行、单实例锁与各 IM 桥内嵌尝试。
summary: orchestration.json、scheduler.lock、IM 桥尝试内嵌 scheduler 与独立 anycode scheduler 二选一夺锁。
read_when:
  - 你需要类似 OpenClaw 的定时 agent 任务。
---

# 定时任务与内置调度器

## 能力说明

1. **工具 `CronCreate` / `CronDelete` / `CronList`**：把规则写入 **`~/.anycode/tasks/orchestration.json`**。表达式为 `cron` crate 语义（6 段：`秒 分 时 日 月 周`；传统 5 段会在前补 `0` 秒，见源码 **`crates/cli/src/scheduler.rs`**）。登记成功时若表达式可解析，响应含 **`next_fire_utc`** / **`next_fire_local`**，便于在首次 tick 前核对时间。**`schedule_timezone`** 仅支持 **`local`**（默认）或 **`utc`**，不支持 IANA 时区名。
2. **`anycode scheduler`**：常驻子命令，读同一 JSON，到点把每条任务的 **`command`** 当作**单次** agent 提示执行（与 `anycode run` 同类）。

**仅写 JSON 不会自动跑**：必须有一个调度循环在跑（见下）。

## 单实例锁 `scheduler.lock`

同机只应有一个调度循环，通过 **`~/.anycode/tasks/scheduler.lock`** 独占锁实现。

- 若已有一个 **`anycode scheduler`** 进程，再启动第二个会在日志中提示锁被占用并退出。
- **微信 / Telegram / Discord 长驻桥**在启动时可 **内嵌** 同一调度循环（`tokio::spawn` → `run_builtin_scheduler`），与 **`anycode scheduler`** 共用 **`scheduler.lock`**：先抢到锁的进程负责 tick；未抢到的静默结束嵌入任务（聊天不受影响，但若本机无任何持锁进程，cron 仍不会触发）。

## 独立运行 `anycode scheduler`（可选）

适合与某一 IM 桥分离、或不跑任何桥时独占调度。可用终端、**tmux**、**systemd user**、**macOS LaunchAgent** 等托管。

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

**`workspace-assistant`** 已暴露 **`CronCreate` / `CronDelete` / `CronList`**。**注册成功不等于已到期执行**；必须由 **某个持锁进程**（`anycode scheduler` **或** 某一 IM 桥成功内嵌的调度循环）按计划触发，且 **同机只有一个**。

English: [Cron & scheduler](/guide/cli-scheduler).
