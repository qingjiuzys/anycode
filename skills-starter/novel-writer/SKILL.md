---
name: novel-writer
description: Long-form fiction workflow — outline, chapter files, and continuation protocol.
description_zh: 长篇小说写作流程：大纲、分章落盘与续写协议。
category: business
---

# novel-writer

> **中文**：大纲 → `chapters/NN.md` 分章落盘 → 按续写协议写下一章。  
> **English**: Outline → chapter files under `chapters/NN.md` → continuation protocol.

## Workflow

1. Agree on genre, tone, target length, and chapter count (default 6–12).
2. Write `novel-outline.md` with chapter titles and one-line summaries.
3. Create `chapters/` and write one chapter per turn as `chapters/01.md`, `chapters/02.md`, …
4. Each chapter file starts with `# Chapter N — Title` then prose (1500–3000 Chinese chars or 800–1500 English words).
5. **Continuation**: when the user says「继续」「下一章」「continue」, read prior chapters + outline, then write the next numbered file only.
6. Keep character names and plot beats consistent; append a 3-bullet recap at the end of each chapter file.

## Notes

- Do not dump the entire novel in chat — persist to files.
- If context is tight, summarize prior chapters in `chapters/_recap.md` before continuing.
- For settings, user may raise `runtime.max_agent_turns` in config for longer sessions.
