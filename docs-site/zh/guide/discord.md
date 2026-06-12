---
title: Discord
description: 通过 Bot Token 与频道 ID 将 anyCode 接入 Discord。
summary: 创建 Bot、邀请到服务器、测试消息并运行桥接。
read_when:
  - 你想在 Discord 里使用 anyCode。
  - 需要频道 ID 或权限说明。
---

# Discord

通过 Discord 机器人在频道内向本机 anyCode 发送任务。

完成后你将了解：

- 如何在开发者门户创建 Bot
- 如何邀请 Bot 并复制频道 ID
- 如何用测试消息验证并启动桥接

## 工作台快速路径

1. 打开 **设置 → 消息渠道**（或设置向导 **渠道 → Discord**）。
2. 在 [开发者门户](https://discord.com/developers/applications) 创建应用与 Bot。
3. 粘贴 Bot Token → **验证连接**。
4. 使用界面 **邀请链接** 将 Bot 加入服务器，授予 **查看频道** 与 **发送消息**。
5. 在 Discord **设置 → 高级** 开启 **开发者模式**，右键频道 → **复制频道 ID**。
6. 粘贴频道 ID → **发送测试消息** → **保存**。
7. 运行（保持终端开启）：

```bash
anycode channel discord
```

凭据：`~/.anycode/channels/discord.json`。

## 命令行

```bash
anycode setup --channel discord
anycode channel discord
```

## 特权 Intent

在 **Bot → Privileged Gateway Intents** 中建议开启 **Message Content Intent**。

## 权限

至少需要：查看频道、发送消息；建议包含读取消息历史。

## 常见错误

| 错误 | 含义 |
|------|------|
| 401 | Token 无效或已重置 |
| 403 | 未邀请 Bot 或无发言/查看权限 |
| 404 | 频道 ID 错误 |

保存前请在工作台 **发送测试消息** 确认。

## 延伸阅读

- [Telegram](./telegram)  
- [微信与配置](./wechat)  
- [通知](./notifications)
