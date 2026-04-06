---
title: 微信与 onboard
description: 首次 onboard 与可选的微信 iLink 桥接说明。
summary: 工作区初始化、API 向导、可选扫码绑定与自启桥。
read_when:
  - 要用手机微信驱动同一套 Agent。
  - 在无界面环境装好后补绑微信。
---

# 微信与 onboard

## `onboard`

一条命令完成：

1. 初始化 **`~/.anycode/workspace`** 等用户目录（与 **`~/.anycode/wechat`** 并列）。
2. 若缺少有效 LLM 配置，进入 **config 向导**（写入 `~/.anycode/config.json`）。
3. 在 TTY 下一般会询问是否 **绑定微信并安装登录自启后台桥**（与 `anycode wechat` 相同流程）。

```bash
./target/release/anycode onboard
./target/release/anycode onboard --skip-wechat
```

**`--debug`**、**`-c/--config`**、**`WCC_DATA_DIR`** 等与 **`wechat`** 子命令一致。

## `wechat`

跳过 onboard 里的微信后，或需要重新绑定时：

```bash
anycode wechat
```

需在能完成 **扫码登录** 的环境（浏览器/图形界面）。

## 用户工作区与微信 `workingDirectory`

从各目录运行 TUI / **`repl`** / **`run`** 时，会把有效工作目录登记到 **`~/.anycode/workspace/projects/index.json`**（按 `last_seen`，约 200 条上限）。任务 cwd 仍是当前目录或 **`-C`**。

**微信桥**：`config.env` 里 **`workingDirectory`** 在新绑定或缺省时默认为上述工作区根的规范路径（避免 LaunchAgent/systemd 下 `current_dir` 为 `/`）；可在微信内用 **`/cwd`** 改为项目目录。

## 下一步

- [run / REPL / TUI](./cli-sessions)  
- [排错](./troubleshooting)  
