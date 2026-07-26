---
name: deep-research
description: Multi-source research with adversarial verification and a cited report. Patterned after Anthropic's deep-research methodology.
description_zh: 多来源深度研究：扇形搜索、交叉验证、对抗性核查，产出带引用的研究报告（对标 Anthropic deep-research 方法论）。
name_zh: 深度研究
category: research
version: 1.0.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
provides_capabilities: [research.deep]
priority: 100
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: true
---

# deep-research

> **中文**：把一个具体问题研究透——多来源搜集、交叉验证、对抗性核查关键论断，最后产出结构化、带引用的报告。
> **English**: Research a specific question thoroughly — fan-out collection, cross-verification, adversarial checking of key claims, then a structured cited report.

## When to use

- 用户需要事实核查、方案调研、市场/技术选型对比、背景研究，且答案不在单一来源中。
- 不适用于：一句话可答的简单事实（直接 WebSearch）、代码库内部问题（用 Grep/Explore）、用户未给出足够具体的问题（先澄清再研究）。

## Workflow

1. **明确问题**：把用户问题改写成 1 句研究目标 + 3-5 个子问题。若问题过宽（如「买什么车好」），先向用户确认预算/场景/地域等约束，最多问 3 个。
2. **扇形搜集**：对每个子问题用不同角度检索（官方文档、权威媒体、一手数据、反方观点），每个子问题至少 3 个独立来源。记录 URL + 摘录 + 时间。
3. **交叉验证**：关键数字/结论必须在 ≥2 个独立来源一致才采信；冲突处保留双方说法并注明。
4. **对抗性核查**：列出报告中的每条关键论断，逐条自问「如果这是错的，最可能错在哪？」——来源是否一手？数据是否过期（注明日期）？是否存在幸存者偏差/利益相关？无法核实的论断降级为「未证实」。
5. **综合成稿**：按「结论先行 → 证据 → 分歧与不确定性 → 建议」结构输出；每条关键论断挂引用编号，文末列全部来源（标题 + URL + 访问日期）。
6. **落盘**：默认保存到 `research/YYYY-MM-DD-主题.md`（用户未要求时也可直接在对话中输出）。

## Quality contract

- 区分「事实（有引用）」「推断（注明推理链）」「观点（注明来源立场）」。
- 引用必须真实可点击；不得伪造 URL 或虚构来源。
- 过期风险：涉及时效的信息标注「截至 YYYY-MM」。
- 不确定性透明：写不清的就说「证据不足」，不用模糊措辞掩盖。

## Failure recovery

- 搜索受限/来源不足 → 明确告知覆盖范围受限，给出已有证据下的最佳结论与待补项。
- 来源互相矛盾且无法裁决 → 并列呈现 + 给出各自可信度评估，不强行选边。
- 问题过大无法穷尽 → 先交付 80% 覆盖的核心报告 + 明确的「未覆盖清单」。
