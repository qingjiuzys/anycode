---
name: file-organizer
description: Organize files in a folder by rules (date, extension, naming patterns).
description_zh: 按日期、扩展名或命名规则整理文件夹中的文件。
name_zh: 文件整理
category: business
version: 1.1.0
mode: instructions
approval: explicit-before-write
channel_capabilities: [files]
permissions:
  read_dirs: [workspace]
  write_dirs: [workspace]
  network: false
---

# file-organizer

> **中文**：按规则（日期、扩展名、命名模式）整理文件夹。  
> **English**: Organize files in a folder by rules (date, extension, naming patterns).

## Safety-first workflow

1. Confirm the exact source directory and organization rule.
2. Produce a dry-run table: `source → destination → reason`.
3. Detect collisions, hidden files, symlinks, and paths outside the authorized directory.
4. Ask for explicit approval before any rename or move.
5. Apply changes in bounded batches and write `file-organizer-manifest.json` with completed operations.
6. Verify every source/destination pair and report skipped files.

## Rules

- Never delete files as part of organization.
- Never follow symlinks outside the authorized directory.
- Preserve extensions and avoid overwriting existing destinations.
- If a batch fails, stop and use the manifest to describe a safe rollback.
