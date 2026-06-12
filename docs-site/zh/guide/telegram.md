---
title: Telegram
description: 通过 Bot Token 与 Chat ID 将 anyCode 接入 Telegram。
summary: 创建机器人、验证 Token、获取 Chat ID 并运行桥接。
read_when:
  - 你想在 Telegram 里使用 anyCode。
  - 创建 Bot 后不知道如何填写 Chat ID。
---

# Telegram

通过 Telegram 机器人，在手机上向本机 anyCode 发送任务。

完成后你将了解：

- 如何用 BotFather 创建机器人
- 如何验证 Token 与获取 Chat ID
- 如何启动桥接

## 工作台快速路径

1. 打开 **设置 → 消息渠道**（或设置向导 **渠道** 步骤）。
2. 按引导：BotFather → 粘贴 Token → **验证连接**。
3. 在 Telegram 中打开机器人并发送 `/start`。
4. 点击 **刷新对话列表**，选择对话后 **保存**。
5. 在终端运行界面显示的桥接命令（需保持运行）：

```bash
anycode channel telegram
```

凭据保存在 `~/.anycode/channels/telegram.json`（Token 仅存本机）。

## 命令行

```bash
anycode setup --channel telegram
anycode channel telegram
```

## 创建机器人

1. 在 Telegram 打开 [@BotFather](https://t.me/BotFather)。
2. 发送 `/newbot` 并按提示操作。
3. 复制 **HTTP API Token**。

## Chat ID

私聊：给机器人发消息后，工作台 **刷新对话列表** 即可选择。

手动方式：对机器人发消息后，用 `getUpdates` 查看 `chat.id`；私聊也可用 `@userinfobot` 等工具查看用户 ID。

**群组** 可能需要 BotFather `/setprivacy` → **Disable**，机器人才能收到群消息。

## 常见问题

| 现象 | 处理 |
|------|------|
| 验证失败 | 检查 Token；泄露请在 BotFather 重置 |
| 对话列表为空 | 先给机器人发 `/start`，再刷新 |
| 群里无响应 | 关闭隐私模式；确认机器人已入群 |
| 桥接退出 | 重新运行 `anycode channel telegram`；检查模型配置 |

## 延伸阅读

- [Discord](./discord)  
- [微信与配置](./wechat)  
- [配置与安全](./config-security)
