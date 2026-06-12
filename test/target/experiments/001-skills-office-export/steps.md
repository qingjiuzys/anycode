1. 检查 `anycode skills list` 中是否包含 `report-to-csv` 和 `md-to-pdf`。
2. 执行 `anycode skills vet md-to-pdf`。
3. 生成固定销售日报 Markdown。
4. 调用 `report-to-csv/run` 生成 `report_table.csv`。
5. 调用 `md-to-pdf/run` 生成 `sales_daily_analysis.pdf`。
6. 校验 CSV 内容和 PDF 文件类型。
