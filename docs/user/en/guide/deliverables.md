---
title: Conversation deliverables
description: How images, PDFs, Office files, and spreadsheets appear in chat.
---

# Conversation deliverables

Plain assistant text stays as Markdown. **Files on disk** (images, video, PDF, Word, Excel, PPT, mind maps) show up as **cards** in the chat and in the sidebar **Artifacts** index.

## What you'll see

| Type | In chat | You can |
|------|---------|---------|
| Images, video | Preview card | Zoom, download |
| PDF | Inline preview | Open original |
| Word / Excel / PPT | Thumbnail + modal preview | Download, reveal in Finder |
| CSV / spreadsheets | Table thumbnail | Full table in modal |
| Mind maps | Interactive map | Export PNG / SVG |

Large Markdown tables (≥3×3) also render as **table cards** for readability.

## Ask the assistant to create files

Just describe what you need:

- “Turn this outline into a slide deck”
- “Export as PDF”
- “Summarize this data in a spreadsheet”

The assistant uses **Skills** to generate files. Cards appear automatically when done.

## Find past files

1. **In the chat** — scroll to deliverable cards from that run
2. **Sidebar → Artifacts** — cross-session file index (final deliverables by default)

## FAQ

**File exists on disk but no card in chat?**  
Refresh the session; ask the assistant to “show X as a deliverable.”

**Blank preview?**  
Some Office files need a `*.preview.html` sidecar; download the original and open locally.

---

<details>
<summary>For Skill authors: declaring deliverables (technical)</summary>

Priority:

1. `artifacts[]` in tool result JSON
2. Sidecar: `foo.xlsx.anycode-artifact.json`
3. Last stdout line: `ANYCODE_ARTIFACT:{...json...}`

Spreadsheet example:

```json
{
  "path": "/abs/path/report.xlsx",
  "kind": "spreadsheet",
  "title": "report.xlsx",
  "preview_path": "/abs/path/workbook.preview.html",
  "inline": true
}
```

Built-in starters (`anycode-ppt`, `anycode-xlsx`, `md-to-pdf`, `mindmap`, etc.) follow this contract.

</details>

简体中文: [会话交付物](/docs/zh/guide/deliverables).
