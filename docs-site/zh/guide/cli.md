---
title: CLI 总览
description: anycode 二进制、全局选项与分主题子文档入口。
summary: 子命令地图；指向 run/REPL/TUI、模型、微信与诊断；HTTP 守护进程已移除说明见 cli-daemon。
read_when:
  - 需要先看清 CLI 文档结构再深入某一子命令。
---

# anyCode CLI 总览

**二进制名：** `anycode`。

## 全局选项

- **`--debug`** — 调试日志。  
- **`-c/--config <PATH>`** — 指定 JSON 配置；若路径显式给出且文件不存在，**报错退出**。  
- **`--model <ID>`** — **仅**在**无子命令默认 TUI** 时使用**长选项**（避免与 **`repl` 的 `-m/--model`** 冲突）。  
- **`--ignore-approval`**（别名 **`--ignore`**，容错 **`--ingroe`**）— **本进程**跳过工具 y/n，**不写回**配置文件。

`run`、`tui`、`repl`、`model`、`channel` 等均按 **`-c`** 路径读写配置。

```bash
anycode config
```

**`security.*`、记忆字段、环境变量**见 [配置与安全](./config-security)。

## 分主题文档

| 主题 | 页面 |
|------|------|
| `run` / `repl` / 全屏 TUI / 任务日志 | [run / REPL / TUI](./cli-sessions) |
| HTTP `daemon`（已移除） | [HTTP 守护进程（已移除）](./cli-daemon) |
| `model` 子命令 | [模型子命令](./cli-model) |
| `list-agents` / `list-tools` / `test-security` | [发现与 test-security](./cli-diagnostics) |
| `setup` / `wechat` | [微信与 setup](./wechat) |
| `enable` / `disable` / `status` / `mode` | 特性开关与路由快照（[版本与特性开关](./releases#runtime-feature-flags)） |
| `workspace` | 项目登记与目录级默认（[路由](./routing)） |

运行时特性名见 `anycode_core::FeatureFlag`（`skills`、`workflows`、`goal-mode` 等）。

## 从源码构建

```bash
cargo build --release
./target/release/anycode --help
```

MCP：加 **`--features tools-mcp`**；环境变量 **`ANYCODE_MCP_COMMAND`**、**`ANYCODE_MCP_SERVERS`** 等见根 README 与 [路线图](./roadmap)。

## 界面语言

见 [配置与安全](./config-security) 中 **`ANYCODE_LANG`** / **`LC_*`** 说明。

English: [CLI overview](/guide/cli).
