---
title: Conversation deliverables
description: How images, video, PDF, Office files, and mind maps appear as cards in chat.
---

# Conversation deliverables

Assistant prose stays Markdown. **Images, video, PDF, presentations, documents, and mind maps** that land on disk show up as **deliverable cards** in the conversation stream and in the Artifacts sidebar (defaults to final deliverables only—workspace scan noise stays folded).

## What you see

| Kind | In chat | Sidebar |
|------|---------|---------|
| Image, video, mind map | Preview Viewer | Open / download |
| PDF | Embedded preview | Same |
| PPTX / DOCX / XLSX / CSV | Lightweight file card (system open / download / copy path) | Same |

## How Skills declare deliverables

Priority (do not rely on the model casually mentioning paths in prose):

1. Tool result JSON: `artifacts: [{ "path", "kind", "title", "inline" }]`
2. Sidecar next to the file: `foo.pptx.anycode-artifact.json`
3. stdout footer: `ANYCODE_ARTIFACT:{...json...}`
4. Extension heuristics: gap-fill only; not inlined by default

Starter skills (`office-pptx`, `md-to-pdf`, `mindmap`) already emit this contract.

## Mind maps

Use Markdown heading outlines (`#` / `##` / `###`). Prefer a filename containing `mindmap`, or set `kind: "mindmap"` explicitly. The workbench renders with markmap and can export MD / SVG / PNG.
