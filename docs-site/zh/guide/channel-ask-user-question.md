---
title: 通道 AskUserQuestion（Telegram）
description: Telegram 桥上 AskUserQuestion 的内联键盘、回调与超时行为。
read_when:
  - 你在 Telegram 使用 anyCode，模型发起了多选题。
---

# 通道 AskUserQuestion（Telegram）

当 Agent 调用 **`AskUserQuestion`** 工具时，Telegram 桥会发送**内联键盘**（点选）。在任务运行期间，你的点击通过 **callback query** 回到运行时。

## 行为说明

- **同一会话一条待定问题**：新问题会替换尚未完成的选题（与其它通道 broker 一致）。
- **同 chat 串行执行**：同一聊天的普通消息会在当前任务之后排队，以便桥仍能 **轮询** 接收按钮回调。
- **超时**：长时间未点选会丢弃该题；工具可能返回错误，模型可退回纯文本指引。
- **限制**：最多 **8** 个选项；Telegram **不支持** `multiSelect`，请保持 `multiSelect` 为 false。

## 回落方式

若按钮不可用，用户可回复数字 **1–N** 对应选项；Telegram 的通道 system prompt 中已提示模型该回落方式。

## 维护者引用

- [ADR 008](https://github.com/qingjiuzys/anycode/blob/main/docs/adr/008-channel-ask-user-question-phasing.md) — 分阶段与范围  
- [Spike 草案](https://github.com/qingjiuzys/anycode/blob/main/docs/ops/channel-ask-user-question-spike.md)  
