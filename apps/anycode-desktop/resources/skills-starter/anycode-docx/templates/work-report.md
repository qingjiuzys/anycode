# 工作汇报 · 2026-W30

## Summary

本周核心进展：Workbench 接入 anycode-docx / anycode-xlsx，文档与表格走「源文件 + HTML 预览」交付链，不再强绑 OOXML 导出。

## Progress

- 完成 FDE Editorial 报告模板三套（周报 / 述职 / 简报）
- validate_report_md 门禁：H1/H2 + Decision/Action + 禁占位符
- Workbench 可直接打开 `report.preview.html` 评审

## Metrics

| 指标 | 本周 | 环比 |
|------|------|------|
| Skill 调用成功率 | 94% | +3pp |
| 产物 validate 通过率 | 88% | +12pp |
| 平均交付轮次 | 2.1 | -0.4 |

## Issues

- 部分模型仍跳过模板自造 Markdown 结构

## Next Steps

- 安装 anycode-docx 到 ~/.anycode/skills
- 用 work-report 模板跑一轮周报冒烟

Decision: 产品组确认默认交付物为 Markdown + preview.html，docx 仅按需导出。

Action: Lin Wei 将在 2026-07-28 前更新 task_compiler 约束文案并同步文档站。
