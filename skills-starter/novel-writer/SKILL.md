---
name: novel-writer
description: Long-form fiction workflow — outline, chapter files, and continuation protocol.
description_zh: 长篇小说写作流程：大纲、分章落盘与续写协议。
name_zh: 小说创作
category: creative
version: 1.1.0
mode: instructions
approval: read-only-unless-writing-output
channel_capabilities: [files, markdown]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# novel-writer

> **English**: Outline → chapter files under `chapters/NN.md` → continuation protocol.
> **中文**：大纲 → `chapters/NN.md` 分章落盘 → 按续写协议写下一章。

## When to use

**Use when:**
- The user wants to write a long-form fiction work (novel, novella, serialized story).
- The user asks to outline, draft chapters, or continue an existing fiction project.

**Do not use when:**
- The user wants a short one-off story or flash fiction (just write it directly).
- The task is non-fiction writing (reports, documentation, essays).
- The user wants to edit an existing published work.

## Workflow

1. Agree on genre, tone, target length, and chapter count (default 6–12).
2. Write `novel-outline.md` with chapter titles and one-line summaries.
3. Create `chapters/` and write one chapter per turn as `chapters/01.md`, `chapters/02.md`, …
4. Each chapter file starts with `# Chapter N — Title` then prose (1500–3000 Chinese chars or 800–1500 English words).
5. **Continuation**: when the user says「继续」「下一章」「continue」, read prior chapters + outline, then write the next numbered file only.
6. Keep character names and plot beats consistent; append a 3-bullet recap at the end of each chapter file.

## Quality contract

- Do not dump the entire novel in chat — persist to files.
- Keep character names, timelines, and plot beats consistent across chapters.
- Each chapter file must include a 3-bullet recap at the end for continuity.
- If context is tight, summarize prior chapters in `chapters/_recap.md` before continuing.
- Match the agreed genre, tone, and style throughout.

## Failure recovery

- If context window is exhausted, write a `chapters/_recap.md` summary of prior chapters and continue from there.
- If the outline becomes inconsistent mid-writing, flag contradictions and propose corrections before proceeding.
- For very long sessions, suggest the user raise `runtime.max_agent_turns` in config.
- If a chapter file cannot be written, report the error and do not skip or reorder chapters silently.
