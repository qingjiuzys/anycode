---
name: video-script
description: Shot-by-shot video script plus asset generation via media tools.
description_zh: 分镜视频脚本，并调用媒体工具生成素材。
category: business
---

# video-script

> **中文**：分镜脚本 + 调用 GenerateImage / GenerateVideo 出素材（非完整剪辑）。  
> **English**: Shot list script + GenerateImage / GenerateVideo assets (not full editing).

## Workflow

1. Clarify audience, length (30–90s), and style (explainer / promo / tutorial).
2. Write `video/script.md` with sections:
   - Hook (0–5s)
   - Scene table: `| # | Visual | VO | Duration |`
   - CTA
3. For 3–6 key scenes, call **GenerateImage** or **GenerateVideo** (when configured) and save paths under `video/assets/`.
4. Deliver `video/script.md` + asset list; note that final edit is out of scope unless user has FFmpeg workflow.

## Notes

- Prefer short clips per scene; do not assume a single long render.
- Match configured `models.video` / `models.image` capabilities.
