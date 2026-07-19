# 07 — md-to-pdf / HTML skill

将 `fixtures/report_table.md` 用本地 skill `md-to-pdf` 导出到 `artifacts/`：

- 优先 PDF；若无 pandoc/PDF 引擎则输出 HTML 也可
- 产物文件名含 `report_table`
- 使用 Skill 工具调用 md-to-pdf
