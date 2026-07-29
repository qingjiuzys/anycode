# DeepSeek 办公技能执行手册

> 适用于 `deepseek-v4-flash` / `deepseek-v4-pro` 及兼容 API。目标：**少猜测、多复制、可验证**。

## 通用原则

1. **先 Read 模板，再 Write** — 禁止跳过 `templates/` 自造结构。
2. **一步一验证** — 填完源文件后必须跑 skill 的 `run`；失败则修源文件重跑，不要手改终稿。
3. **终稿清单** — 回复里列绝对路径：必须交付物 + 可选预览物。
4. **禁止占位** — 无 TBD / lorem / 待填 / xxx；数字要具体。
5. **外部数据** — 统计、政策、行情必须先查再写；表格加 `Source Name` + `Source URL` 列（plain text，不用 HYPERLINK）。

## 按技能

| 技能 | 源 | 终稿 | run 后必查 |
|------|-----|------|-----------|
| anycode-ppt | 复制 `templates/*.html` | `slides/*.html` + `index.html` | validate 通过、≥2 页 |
| anycode-docx | 复制 `templates/*.md` | `.docx` + `.preview.html` | Decision/Action 行存在 |
| anycode-xlsx | 复制 `templates/workbook-*.json` | `.xlsx` | recheck 无公式错误、≥3 sheet |
| anycode-pdf | 复制 `templates/*.md` | `.pdf` + `.preview.html` | PDF 非空、中文文档用 GB/T 7714 引用格式 |

## 常见失败 recovery

- **validate 报错** → 对照 `components.md` 补章节/表头，勿删门禁字段。
- **xlsx recheck 报 #REF! / #NAME?** → 修 workbook.json 公式后重新 `run`。
- **pdf 引擎缺失** → 安装 `playwright` + chromium；或交付 preview.html 并说明。
