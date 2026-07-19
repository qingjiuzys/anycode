---
name: cn-meeting-minutes
description: Turn meeting notes or transcripts into structured Chinese meeting minutes.
description_zh: 将会议记录或转写文本整理为结构化中文会议纪要。
name_zh: 会议纪要
category: business
version: 1.1.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# cn-meeting-minutes

> **中文**：从原始笔记、录音转写或聊天摘录生成规范会议纪要。  
> **English**: Structure raw notes or transcripts into Chinese meeting minutes.

## 输出模板

1. **会议主题 / 时间 / 参与人**
2. **讨论要点** — 分议题 bullet
3. **决议事项**
4. **Action items** — 负责人 + 截止日期（未知则 TBD）
5. **附录**（可选）— 未决问题

## 规则

- 不捏造未出现的决议；转写不清处标 `[待确认]`。
- 若用户提供录音且配置了 STT，可先转写再整理。
- 语气正式、中性；避免口语堆砌。

## 适用边界

- 适用于会议笔记、录音转写、群聊讨论整理。
- 不适用于周报、新闻摘要或凭空生成会议结论。

## 执行步骤

1. 识别会议主题、时间、参与人和原始材料范围。
2. 按议题归并重复表述，保留不同意见和未决事项。
3. 将行动项整理为 `事项 / 负责人 / 截止时间 / 状态` 表格。
4. 无法确认的信息统一标记 `[待确认]`。
5. 默认在对话中输出；需要落盘时建议保存到 `minutes/YYYY-MM-DD-主题.md`。

## 质量与恢复

- 决议、负责人和日期必须能在原始材料中找到依据。
- 转写内容缺失时先生成“纪要草稿”，并单列缺失信息，不阻塞其余内容交付。
