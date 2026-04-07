---
title: 微信与 setup
description: 首次 setup 与可选的微信 iLink 桥接说明。
summary: 工作区初始化、模型配置、channel 选择与扫码绑定自启桥。
read_when:
  - 要用手机微信驱动同一套 Agent。
  - 在无界面环境装好后补绑微信。
---

# 微信与 setup

## `setup`

一条命令完成：

1. 初始化 **`~/.anycode/workspace`** 等用户目录（与 **`~/.anycode/wechat`** 并列）。
2. 若缺少有效 LLM 配置，进入 **config 向导**（写入 `~/.anycode/config.json`）。
3. 在 TTY 下会先选择 channel（`wechat` / `telegram` / `discord`），再进入对应流程。

```bash
./target/release/anycode setup
./target/release/anycode setup --channel wechat
```

**`--debug`**、**`-c/--config`**、**`WCC_DATA_DIR`** 等与 **`channel wechat`** 子命令一致。

## `channel wechat`

跳过 setup 里的微信后，或需要重新绑定时：

```bash
anycode channel wechat
```

需在能完成 **扫码登录** 的环境（浏览器/图形界面）。

## 用户工作区与微信 `workingDirectory`

从各目录运行 TUI / **`repl`** / **`run`** 时，会把有效工作目录登记到 **`~/.anycode/workspace/projects/index.json`**（按 `last_seen`，约 200 条上限）。任务 cwd 仍是当前目录或 **`-C`**。

**微信桥**：`config.env` 里 **`workingDirectory`** 在新绑定或缺省时默认为上述工作区根的规范路径（避免 LaunchAgent/systemd 下 `current_dir` 为 `/`）；可在微信内用 **`/cwd`** 改为项目目录。

## 下一步

- [run / REPL / TUI](./cli-sessions)  
- [排错](./troubleshooting)  
