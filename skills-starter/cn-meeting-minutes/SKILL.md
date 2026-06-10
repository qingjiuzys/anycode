---
name: cn-meeting-minutes
description: Turn meeting notes or transcripts into structured Chinese meeting minutes.
description_zh: 将会议记录或转写文本整理为结构化中文会议纪要。
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
