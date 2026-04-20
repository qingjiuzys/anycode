# 通道 AskUserQuestion：能力边界与设计草案（spike）

## 各通道交互能力（概要）

| 通道 | 内联按钮 / 键盘 | 文本 fallback |
|------|------------------|---------------|
| **Telegram** | Bot API 支持 `InlineKeyboardMarkup`、callback query | 可回落为纯文本指令 |
| **Discord** | 组件（Button、Select）与交互 token | 可回落为文本 |
| **微信（iLink 桥）** | 当前桥以 **`send_text`** 为主；审批为 **y/n** 文本 | 适合超时 + 明确文本提示 |

## 目标

在通道上与 TUI **`AskUserQuestionHost`** 类似：**结构化选项 / 超时 / 与审批状态机可组合**，但不急于引入新的 **public trait**；优先 **枚举**、`pub(crate)` 宿主或单一 broker 扩展点。

## 数据面（草案）

- **`question_id`**：UUID，与 `PermissionBroker` 或独立 `QuestionBroker` 关联。
- **`prompt`**：展示给用户的短文案（多语言可由 Fluent 统一）。
- **`options`**：`[{ "id": "a", "label": "…" }, …]`；微信可渲染为 `回复 a / b` 或 `1 / 2`。
- **`timeout`**：与现有微信审批超时一致时可复用 broker 定时器语义。

## 与 PermissionBroker 的关系

- **敏感工具审批**：已有 pending + y/n 解析。
- **AskUserQuestion**：更偏「产品化多选 / 表单」；可 **共用同一消息路由**（同一会话互斥），或 **分层**：工具审批优先、Question 队列其次；需避免两路 pending 死锁（spike 结论：**同一 chat 同一时刻仅一种 pending**）。

## 后续

若产品化，建议补 **ADR**：超时默认值、Discord/Telegram 组件载荷映射、与 `SecurityLayer` 回调的边界。
