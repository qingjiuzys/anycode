---
title: 会话交付物
description: 图片、PDF、Office、表格与思维导图如何在对话中展示。
---

# 会话交付物

助手写的说明仍用 Markdown 显示；**落盘的文件**（图片、视频、PDF、Word、Excel、PPT、思维导图等）会以**卡片**出现在对话里，并同步到侧栏 **产物**。

## 你会看到什么

| 文件类型 | 对话里 | 你可以 |
|----------|--------|--------|
| 图片、视频 | 预览卡片 | 放大、下载 |
| PDF | 内嵌预览 | 打开原文件 |
| Word / Excel / PPT | 缩略图 + 弹窗预览 | 下载、在 Finder 中打开 |
| CSV / 表格 | 表格缩略图 | 弹窗查看全表 |
| 思维导图 | 可交互导图 | 导出 PNG / SVG |

正文里较大的 Markdown 表格（≥3 行 × ≥3 列）也会变成**表格卡片**，方便阅读。

## 怎么让助手生成交付物

在对话里直接说需求即可，例如：

- 「把这份大纲做成 PPT」
- 「导出为 PDF」
- 「用表格汇总这些数据」

助手会通过 **Skills**（内置技能包）生成文件。生成完成后，卡片会自动出现在对话流中。

## 在哪里找历史文件

1. **对话内** — 向上滚动查看当次生成的卡片
2. **侧栏 → 产物** — 跨会话的文件索引（默认只显示最终交付物）

## 常见问题

**卡片没出现，但文件已经在文件夹里？**  
刷新会话；若仍没有，可能是该文件未被标记为交付物。重新说明「请把 xxx 作为交付物展示」。

**预览是空白？**  
部分 Office 文件需要同目录下的 `*.preview.html` 侧车文件；可下载原文件用本地应用打开。

---

<details>
<summary>给 Skill 作者：如何声明交付物（技术说明）</summary>

优先级：

1. 工具结果 JSON 中的 `artifacts[]`
2. 同目录 sidecar：`foo.xlsx.anycode-artifact.json`
3. stdout 末行：`ANYCODE_ARTIFACT:{...json...}`

表格类示例：

```json
{
  "path": "/abs/path/report.xlsx",
  "kind": "spreadsheet",
  "title": "report.xlsx",
  "preview_path": "/abs/path/workbook.preview.html",
  "inline": true
}
```

内置 starter（`anycode-ppt`、`anycode-xlsx`、`md-to-pdf`、`mindmap` 等）已按此约定发射。

</details>

English: [Conversation deliverables](/docs/guide/deliverables).
