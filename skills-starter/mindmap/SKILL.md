---
name: mindmap
description: Turn a topic or notes into a Markdown heading outline for mind-map viewing in anyCode.
channel_capabilities: [files, artifacts]
---

# Mind map

Write a Markdown file whose `#` / `##` / `###` headings form a mind-map tree.

## Steps

1. Create `mindmap-<slug>.md` in the project (or cwd).
2. Use one `#` root and nested `##` / `###` branches (no tables required).
3. Print the absolute path, then emit:

```
ANYCODE_ARTIFACT:{"path":"/abs/path/mindmap-foo.md","kind":"mindmap","title":"…","inline":true}
```

4. Also write `<path>.anycode-artifact.json` with the same JSON object.

anyCode renders `kind=mindmap` as an interactive outline card in the conversation.
