---
name: pdf
description: Read, merge, split, extract text/tables from, and fill PDF forms. Adapted from Anthropic's document skills patterns.
description_zh: 读取、合并、拆分、提取文本/表格、填写 PDF 表单（参考 Anthropic 官方文档技能模式）。
name_zh: PDF 处理
category: office
version: 1.0.0
mode: executable
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
provides_capabilities: [pdf.read, pdf.manipulate, pdf.form]
priority: 100
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# pdf

> **中文**：PDF 全能处理——提取文本与表格、合并/拆分/旋转页面、填写表单域、生成简单 PDF。
> **English**: Full PDF handling — text/table extraction, merge/split/rotate, form filling, simple generation.

## When to use

- 用户给出 `.pdf` 文件并要求阅读、总结、抽取数据、转换格式。
- 需要合并多个 PDF、拆分章节、旋转/删除页面。
- 需要填写可填写表单（AcroForm）或基于模板生成 PDF。
- 不适用：扫描件 OCR（无文本层时先说明需要 OCR，可调用 STT/OCR 能力若可用）、复杂版式重排（建议改走 anycode-pdf 或 anycode-docx）。

## Workflow

1. **Inspect first**：用 `run info <file>` 查看页数、是否加密、是否含表单域/文本层。
2. **Extract**：
   - 文本：`run text <file> [--pages 1-3]`
   - 表格：优先 `pdfplumber`（`run tables <file>`），无 pdfplumber 时退化为文本抽取并说明。
3. **Manipulate**：
   - 合并：`run merge a.pdf b.pdf -o out.pdf`
   - 拆分：`run split <file> --pages 2-5 -o part.pdf`
   - 旋转：`run rotate <file> --pages 1 --angle 90 -o out.pdf`
4. **Forms**：`run fields <file>` 列出表单域；`run fill <file> --json data.json -o filled.pdf`（填写后保留可编辑表单域并刷新外观；如需真正扁平化，用 `run merge filled.pdf -o flat.pdf` 前先用 `qpdf --flatten-annotations` 或打印导出）。
5. **Generate**（简单场景）：用 reportlab 写段落/表格；中文必须注册 CJK 字体（run 脚本内置 `STSong-Light`）。

## Quality contract

- 抽取内容必须标注来源页码；找不到时明确说明，**不得编造**。
- 加密 PDF 需要密码时向用户索取，不尝试爆破。
- 输出文件写入工作区 `output/` 或用户指定路径，完成后回报绝对路径与页数。
- 合并/拆分前后校验页数守恒（sum(inputs) == pages(output)），不一致时报告。

## Failure recovery

- 无文本层（`text` 输出为空）→ 提示扫描件，建议 OCR 或提供文字版。
- 缺 pypdf/reportlab/pdfplumber → `pip install pypdf reportlab pdfplumber`（或 `uv run --with pypdf ...`），失败则降级为纯文本方案并说明受限项。
- 表单域名未知 → 先 `run fields` 列出，再与用户字段做映射，映射结果随输出一起给出。
