---
name: video-script
description: Shot-by-shot video script plus asset generation via media tools.
description_zh: 分镜视频脚本，并调用媒体工具生成素材。
name_zh: 视频脚本
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

# video-script

> **English**: Shot list script + GenerateImage / GenerateVideo assets (not full editing).
> **中文**：分镜脚本 + 调用 GenerateImage / GenerateVideo 出素材（非完整剪辑）。

## When to use

**Use when:**
- The user wants a shot-by-shot video script with scene breakdowns.
- The user needs asset generation (images, short clips) for key scenes.
- Target length is typically 30–90 seconds (explainer, promo, tutorial).

**Do not use when:**
- The user wants a full edited video (out of scope unless they have an FFmpeg workflow).
- The task is a faceless explainer video (use faceless-explainer).
- The task is a product launch video, PR-to-video, or website-to-video (use dedicated skills).
- The user only wants a text outline without scene breakdown.

## Workflow

1. Clarify audience, length (30–90s), and style (explainer / promo / tutorial).
2. Write the script Markdown (default `video/script.md`, or the path the user gives) with:
   - Hook (0–5s)
   - Scene table: `| # | Visual | VO | Duration |`
   - CTA
3. For 3–6 key scenes, call **GenerateImage** or **GenerateVideo** (when configured) and save under `video/assets/` (or the user-specified assets dir).
4. Deliver script + asset list; final edit is out of scope unless the user has an FFmpeg workflow.

## Quality contract

- Only mark an asset ✅ / "generated" if the file **exists on disk** after the tool call.
- If GenerateImage/GenerateVideo returns 402/error/empty path, mark the row **FAILED** with the error snippet — never invent paths.
- Do not claim CDN URLs were saved locally unless you actually downloaded them.
- Prefer short clips per scene; do not assume a single long render.
- Match configured `models.video` / `models.image` capabilities.

## Failure recovery

- If GenerateImage/GenerateVideo is unavailable or returns errors, mark affected scenes as **FAILED** with error details and deliver the script without those assets.
- If the user has no media generation tools configured, produce the script only and suggest installing the required tools.
- If context is insufficient to design scenes, ask the user for more detail on audience, tone, or key messages before proceeding.
