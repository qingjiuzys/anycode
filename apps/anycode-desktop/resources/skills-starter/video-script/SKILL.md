---
name: video-script
description: Shot-by-shot video script plus asset generation via media tools.
description_zh: 分镜视频脚本，并调用媒体工具生成素材。
name_zh: 视频脚本
category: business
version: 1.1.0
---

# video-script

> **中文**：分镜脚本 + 调用 GenerateImage / GenerateVideo 出素材（非完整剪辑）。  
> **English**: Shot list script + GenerateImage / GenerateVideo assets (not full editing).

## Workflow

1. Clarify audience, length (30–90s), and style (explainer / promo / tutorial).
2. Write the script Markdown (default `video/script.md`, or the path the user gives) with:
   - Hook (0–5s)
   - Scene table: `| # | Visual | VO | Duration |`
   - CTA
3. For 3–6 key scenes, call **GenerateImage** or **GenerateVideo** (when configured) and save under `video/assets/` (or the user-specified assets dir).
4. Deliver script + asset list; final edit is out of scope unless the user has an FFmpeg workflow.

## Quality contract (mandatory)

- Only mark an asset ✅ / “generated” if the file **exists on disk** after the tool call.
- If GenerateImage/GenerateVideo returns 402/error/empty path, mark the row **FAILED** with the error snippet — never invent paths.
- Do not claim CDN URLs were saved locally unless you actually downloaded them.

## Notes

- Prefer short clips per scene; do not assume a single long render.
- Match configured `models.video` / `models.image` capabilities.
