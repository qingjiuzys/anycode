---
title: 快速开始
description: 约五分钟：安装 → setup → run / TUI / repl 验证。
summary: 最短上手路径；指向安装、CLI 分册与文档地图。
read_when:
  - 第一次使用 anyCode，想尽快跑通。
---

# 快速开始

## 五分钟路径

1. **安装** — 见 [安装](./install)（一行脚本、Release 或源码构建）。  
2. **`setup`** — 工作区 + API 向导 + TTY 下可选微信。  
3. **验证** — 跑一次 `run`，或进入默认 TUI / `repl`。

## 环境要求

- **只用预编译**：无需 Rust。  
- **从源码构建**：需要 **Rust（stable）** 与 **Cargo**。  
- **微信扫码**：需支持图形界面/浏览器的环境（见 [微信与 setup](./wechat)）。

## 首次运行

```bash
./target/release/anycode setup
./target/release/anycode setup --skip-wechat
```

## 验证

```bash
anycode run --agent general-purpose "请只回复：OK"
anycode
```

TUI 中：`/help`、`/agents`、`/tools`、`/exit`。需要**原生终端滚动**时用 **`anycode repl`**，见 [run / REPL / TUI](./cli-sessions)。

## 界面语言

**`ANYCODE_LANG`**、`LANG` / `LC_MESSAGES` 或系统语言；可强制：

```bash
export ANYCODE_LANG=en
export ANYCODE_LANG=zh
```

## 下一步

- [文档地图](./docs-directory)  
- [CLI 总览](./cli)  
- [模型与端点](./models)  
- [架构](./architecture)  

English: [Getting started](/guide/getting-started).
