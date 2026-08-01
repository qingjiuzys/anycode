---
name: content-repurpose
description: Repurpose long content into shorter posts or alternate formats (drafts only, never published).
description_zh: 将长文/纪要/报告改写为短文帖、线程、大纲等目标格式（仅生成草稿，绝不发布）。
name_zh: 内容改写
category: writing
version: 1.0.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# content-repurpose

> **中文**：把一份源材料改写成用户要的目标形态（短帖、推文线程、大纲、摘要、脚本），保留事实与立场，只产出草稿。
> **English**: Rewrite source material into the target format the user requests (short post, thread, outline, summary, script). Drafts only.

## When to use

- 用户有现成材料（文章、会议纪要、报告、转写）并希望换一种形态分发或复用。
- 不适用于：凭空创作（用对应写作 skill）、翻译（直接翻译即可）、发布到外部平台（本 skill 绝不发布）。

## Workflow

1. **读取源材料**：确认来源路径/内容范围；材料缺失或过短时向用户索取，不自行补写事实。
2. **确认目标形态与约束**：目标格式（短帖/线程/大纲/摘要/视频脚本）、长度、语气、受众、平台特征（如短帖要钩子开头、线程要分条）。未知则给合理默认并说明。
3. **抽取骨架**：先提炼核心论点、关键数据、结论，再按目标形态重组；数据与事实必须能在源材料中找到出处。
4. **产出草稿**：默认 Markdown 文件存到 `drafts/`（或在对话中输出），文件头注明「草稿 · 源自 `<path>` · 未发布」。
5. **交付说明**：回报改写了什么、省略了什么、哪些点需要用户确认。

## Quality contract

- **不发布**：任何外部平台发布动作都需要用户明确另行指示；本 skill 只做草稿。
- 不新增源材料中没有的事实、数字、引用；必须压缩时优先保留数据与结论。
- 语气转换可调整表达，不得扭曲原意或立场；有歧义处保留原措辞并标注。
- 长度硬约束（如 280 字）必须达标，超出时继续压缩而非截断。

## Failure recovery

- 源材料与目标形态冲突（如数据密集报告改极短帖）→ 给出压缩版 + 「被省略的关键信息」清单，由用户取舍。
- 源材料质量差（缺结论、逻辑断裂）→ 先产出「草稿 + 缺口清单」，不虚构补全。
