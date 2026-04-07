---
title: 快速开始
description: 安装 anyCode，完成 setup，并在几分钟内跑通第一条任务。
summary: 面向非技术用户的最短路径，包含失败时下一步动作。
read_when:
  - 第一次使用 anyCode，想尽快跑通。
---

# 快速开始

适合第一次使用 anyCode 的用户。

完成本页后，你会得到：

- anyCode 已安装
- `setup` 已完成
- 一条验证任务成功返回

## 五分钟路径

1. **安装** — 见 [安装](./install)。  
2. **运行 `setup`** — 选择模型与 channel（`wechat` / `telegram` / `discord`）。  
3. **验证** — 执行一次任务并检查输出。

## 环境要求

- **使用预编译安装**：不需要 Rust。
- **仅当你从源码构建时**：需要 Rust + Cargo。
- **微信扫码登录**：需要可打开浏览器/图形界面的机器。

## 首次运行（setup）

如果 `anycode` 已经在 PATH 中：

```bash
anycode setup
```

预期输出：进入 setup 向导，提示选择模型和 channel。  
下一步：完成向导后执行下方“验证”命令。

如果你在本地源码目录里直接运行编译产物：

```bash
./target/release/anycode setup
```

预期输出：与 `anycode setup` 相同的向导流程。  
下一步：若 PATH 已配置，后续优先使用 `anycode`。

也可以直接指定 channel：

```bash
anycode setup --channel wechat
anycode setup --channel telegram
anycode setup --channel discord
```

预期输出：跳过 channel 选择菜单，直接进入对应流程。

## 验证

```bash
anycode run --agent general-purpose "请只回复：OK"
anycode
```

预期输出：第一条命令返回 `OK`；第二条命令进入 TUI。

TUI 里可先试：`/help`、`/tools`、`/exit`。

## 失败时下一步

- 提示 `command not found` -> 看 [安装](./install) 里的 PATH 处理
- `setup` 不能交互 -> 换到本机真实终端执行
- 微信扫码失败 -> 在有图形界面的机器执行 `anycode channel wechat`

## 界面语言

**`ANYCODE_LANG`**、`LANG` / `LC_MESSAGES` 或系统语言；可强制：

```bash
export ANYCODE_LANG=en
export ANYCODE_LANG=zh
```

## 下一步

- [安装](./install)
- [微信与 setup](./wechat)
- [排错](./troubleshooting)
- [文档地图](./docs-directory)

English: [Getting started](/guide/getting-started).
