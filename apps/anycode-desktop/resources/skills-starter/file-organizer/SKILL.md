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

> **English**: Organize files in a folder by rules (date, extension, naming patterns).
> **中文**：按规则（日期、扩展名、命名模式）整理文件夹。

## When to use

**Use when:**
- The user wants to sort, categorize, or reorganize files in a directory.
- Organization rules include date-based, extension-based, or naming-pattern-based grouping.

**Do not use when:**
- The user wants to delete files (this skill never deletes).
- The task involves files outside the authorized workspace directory.
- The user wants to reorganize git-tracked source code (suggest git mv instead).

## Workflow

1. Confirm the exact source directory and organization rule with the user.
2. Produce a dry-run table: `source → destination → reason`.
3. Detect collisions, hidden files, symlinks, and paths outside the authorized directory.
4. Ask for explicit approval before any rename or move.
5. Apply changes in bounded batches and write `file-organizer-manifest.json` with completed operations.
6. Verify every source/destination pair and report skipped files.

## Quality contract

- Never delete files as part of organization.
- Never follow symlinks outside the authorized directory.
- Preserve extensions and avoid overwriting existing destinations.
- Require explicit user approval before any write operation.
- Write a manifest (`file-organizer-manifest.json`) recording every completed operation for auditability.

## Failure recovery

- If a batch fails, stop and use the manifest to describe a safe rollback.
- If collisions are detected, report them and skip conflicting files rather than overwriting.
- If the source directory does not exist or is not accessible, report the error and ask the user to verify the path.
- If symlinks point outside the authorized directory, skip them and list them separately.
