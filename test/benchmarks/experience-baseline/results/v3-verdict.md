# Experience Pack v3 判定

## 结论

以 GPT-5.6 gold 作为外部质量上界后，DeepSeek V4 Flash 的平均分为 **4.0833/5**，加入 `experience@0.2.0` 后为 **4.3000/5**，净提升 **+0.2167**。相对 teacher 的平均差距从 **0.9167** 缩小到 **0.7000**，关闭了约 **23.64%** 的质量差距。

这说明 experience pack 有正向效果，但不是全面增益：它显著修复了 slugify 的失败测试和 PPTX 的重复 JSON key；对 DOCX 与 DDL 仅带来小幅改善；在 landing page 与 cohort SQL 上反而略有退化。Pack 更像是提高约束遵循和交付可靠性的 guardrail，而不是稳定提升视觉创意或已经接近满分的简单 SQL。

## Why V4 vs V4 Pro was the wrong experiment

V4 Flash 与 V4 Pro 属于同一模型家族，往往共享相近的 checklist ceiling；Pro 主要增加推理成本和延迟，却不一定形成稳定、可解释的外部质量跨度。用两者互比容易把同家族风格差异误当成 experience pack 的效果，也缺乏独立质量锚点。

本次 **GPT-5.6 gold vs Flash vs Flash+pack** 更有效：GPT gold 提供外部上界，Flash 是固定低成本基线，唯一主要变量是 experience pack。因而可直接观察 pack 关闭了多少 teacher gap，而不是比较两个相近模型谁更像自己的参考答案。

## 关键证据

- `scene.code.slugify`：原始 Flash 的第 4 个测试断言与实现/题意冲突，测试必失败；enhanced 修复后提升 **+0.9**。
- `scene.office.pptx`：原始 Flash 在 Plan 中重复 `bullets` key，JSON 语义不唯一；enhanced 输出有效，提升 **+0.7**。
- `scene.web.landing`：enhanced 丢失原始 Flash 的终端展示和更丰富视觉层次，下降 **-0.4**；两者也都因 Google Fonts 外链而不完全 self-contained。
- `scene.sql.cohort`：两份学生答案都近乎 gold；enhanced 仅因 Markdown 围栏和较弱可读性小幅落后 **-0.1**。

总体判断：**experience@0.2.0 有可测但中等的净收益，主要价值是减少硬性失误；下一版应重点避免对视觉具体性和简单任务输出纯净度的负迁移。**
