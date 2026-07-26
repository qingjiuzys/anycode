---
title: 无头守护进程
description: anycode-daemon 在无桌面环境下运行 cron 调度。
---

# 无头守护进程（`anycode-daemon`）

独立二进制 **`anycode-daemon`** 承接已退役终端 `anycode` CLI 中的常驻服务：

| 子命令 | 用途 |
|--------|------|
| `scheduler` | Cron / 自动化触发循环 |

安装见 [安装](./install)（Linux/Windows 预编译包或从源码 `cargo install`）。

## 示例

```bash
anycode-daemon scheduler
```

配置位于 `~/.anycode/config.json`（与 Workbench 相同）。首次模型配置请在 Workbench **`/setup`** 完成 — 桌面应用或内嵌 dashboard。

## Workflow 定时任务（DAG）

在 `~/.anycode/tasks/orchestration.json` 的 cron job 上加 `workflow` 字段（指向一个 workflow YAML/JSON 定义文件，相对路径基于 job 工作目录），调度器即按 **DAG（`depends_on` 分层 + checkpoint 断点续跑）** 执行该 workflow，而不是单提示任务；`command` 作为 workflow 的用户提示。定义校验：`depends_on` 必须指向已有步骤、无环（ADR 014 §6）。

```json
{
  "crons": [{
    "id": "nightly-report",
    "schedule": "0 0 3 * * *",
    "command": "生成昨日运维日报",
    "workflow": "workflows/nightly-report.yaml"
  }]
}
```

## 桌面 vs 守护进程

| 场景 | 选择 |
|------|------|
| macOS 日常使用 | **anyCode.app**（工作台 + 原生 STT/OCR） |
| Linux 服务器 / NAS | **`anycode-daemon`** 跑调度 |
| 仅定时任务 | `anycode-daemon scheduler`（或保持桌面应用运行） |

旧版 HTTP `anycode daemon`（POST `/v1/tasks`）已移除 — 见 [ADR 003](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/003-http-daemon-deprecated.md)。

## 相关

- [定时提醒](./cli-scheduler)
- [安装](./install)

English: [Headless daemon](/guide/daemon).
