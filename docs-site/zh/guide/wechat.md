---
title: 微信与 setup
description: 首次 setup 与可选的微信 iLink 桥接说明。
summary: 先判断该用哪个命令，再用最短步骤完成微信接入。
read_when:
  - 要用手机微信驱动同一套 Agent。
  - 在无界面环境装好后补绑微信。
---

# 微信与 setup

适合希望“在微信里发消息，在 anyCode 里执行任务”的用户。

完成本页后，你会知道：

- 先执行哪个命令
- 如何最短路径完成微信绑定
- 扫码失败或目录不对时怎么处理

## 我该用哪个命令？

- 第一次配置 -> `anycode setup`
- 只绑定/重绑微信 -> `anycode channel wechat`
- 想用 Telegram/Discord -> `anycode setup --channel telegram|discord`

## `setup`

推荐首次先执行：

1. 检查并初始化工作目录
2. 需要时补齐模型配置
3. 选择 channel（`wechat` / `telegram` / `discord`）

```bash
anycode setup
anycode setup --channel wechat
```

预期输出：进入 setup 流程并进入模型+channel 配置。

## `channel wechat`

以下场景用它：

- setup 里跳过了微信
- 更换了机器/账号，需要重绑

```bash
anycode channel wechat
```

预期输出：启动微信扫码绑定流程。

需在能完成 **扫码登录** 的环境（浏览器/图形界面）。

## 常见问题

如果微信里执行任务不在你的项目目录，先在微信中执行 `/cwd` 指到项目目录。
预期输出：之后任务会在你指定的项目目录执行。

## 进阶说明

- 微信数据目录通常是 `~/.anycode/wechat`
- 工作区兜底目录通常是 `~/.anycode/workspace`
- 进阶参数（`--debug`、`-c/--config`、`WCC_DATA_DIR`）按 CLI 全局规则生效

## 下一步

- [run / REPL / TUI](./cli-sessions)  
- [排错](./troubleshooting)

