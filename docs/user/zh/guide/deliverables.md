---
title: 会话交付物
description: 图片、视频、PDF、Office 与思维导图如何在对话中以卡片展示。
---

# 会话交付物

助手散文仍用 Markdown；**图片 / 视频 / PDF / 演示文稿 / 文档 / 思维导图**等落盘文件会以**交付物卡片**出现在对话流中，并同步进入侧栏「产物」索引（默认只显示最终交付物，不含工作区扫描噪声）。

## 你会看到什么

| 类型 | 对话内 | 侧栏 |
|------|--------|------|
| 图片、视频、思维导图 | 可预览的 Viewer 卡片 | 打开 / 下载 |
| PDF | 内嵌预览 | 同上 |
| PPTX / DOCX / XLSX / CSV | 轻量文件卡（系统打开 / 下载 / 复制路径） | 同上 |

## Skill 如何声明交付物

优先级（勿依赖模型在正文里随口写路径）：

1. 工具结果 JSON：`artifacts: [{ "path", "kind", "title", "inline" }]`
2. 同目录 sidecar：`foo.pptx.anycode-artifact.json`
3. stdout 末行：`ANYCODE_ARTIFACT:{...json...}`
4. 扩展名启发式：仅作补漏，默认不进对话卡

内置 starter（`office-pptx`、`md-to-pdf`、`mindmap`）已按上述约定发射。

## 思维导图

使用 Markdown 标题大纲（`#` / `##` / `###`），文件名建议含 `mindmap`，或显式 `kind: "mindmap"`。工作台用 markmap 渲染，可导出 MD / SVG / PNG。
