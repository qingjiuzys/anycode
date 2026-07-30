---
name: mindmap
description: Turn a topic or notes into a Markdown heading outline rendered as an interactive mind-map card in anyCode.
description_zh: 将主题或笔记整理为 Markdown 标题树，在 anyCode 会话中渲染为可交互思维导图卡片。
name_zh: 思维导图
category: business
version: 1.0.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, artifacts]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# mindmap

> **中文**：把主题/材料结构化为标题树（`#`/`##`/`###`），anyCode 会渲染成可交互导图卡片。
> **English**: Structure a topic into a heading tree (`#`/`##`/`###`); anyCode renders it as an interactive mind-map card.

## When to use

- 用户要做头脑风暴梳理、知识结构图、项目拆解、读书笔记的结构化呈现。
- 不适用于：流程图/时序图（建议 Mermaid）、纯文本大纲（直接写列表即可）。

## Steps

1. **定根**：一个 `#` 根节点 = 主题本身（不超过 15 字）。
2. **分支**：`##` 为主分支（3-7 个，MECE 优先），`###` 为子要点；每条尽量一行、短语化，不写长句。
3. **取材**：若基于用户材料，节点必须能在材料中找到依据；扩展节点标注 `(扩展)`。
4. 在项目目录（或 cwd）创建 `mindmap-<slug>.md`。
5. 打印绝对路径，然后输出产物标记：

```
ANYCODE_ARTIFACT:{"path":"/abs/path/mindmap-foo.md","kind":"mindmap","title":"…","inline":true}
```

6. 同时写入 `<path>.anycode-artifact.json`，内容为同一 JSON 对象。

anyCode 会将 `kind=mindmap` 渲染为会话内的可交互大纲卡片。

## Quality contract

- 层级 ≤ 4 层；同一父节点下子节点 2-7 个，失衡时重新归类。
- 节点用名词短语，不用完整句子；重复概念合并。
- 不捏造材料外的「事实型」分支；推测性扩展显式标注。

## Failure recovery

- 材料过于零散无法归类 → 先给「候选分支清单」让用户确认，再生成导图。
- 主题过大 → 拆成多张子导图（各自独立文件），根节点互相引用。
