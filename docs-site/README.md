# anyCode documentation site

VitePress sources under this directory. **English** is the default locale (`/`); **简体中文** lives under `/zh/`.

## Commands

```bash
npm install
npm run dev    # local preview
npm run build  # static output → .vitepress/dist (also validates internal links)
npm run preview
```

## GitHub Pages

- **公开站点**（推送 `docs-site/**` 到 `main`/`master` 后由 Actions 构建）：**https://qingjiuzys.github.io/anycode/**  
- 在仓库 **Settings → Pages** 中，**Build and deployment → Source** 请选择 **GitHub Actions**（与 `.github/workflows/docs.yml` 配套）。
- 生产构建使用子路径 **`/anycode/`**（环境变量 **`VITEPRESS_BASE=/anycode/`**）。本地开发 **`npm run dev`** 仍用 **`base: '/'`**，无需设该变量。
- 若需本地验证与线上一致的静态包：`VITEPRESS_BASE=/anycode/ npm run build`，再用 `npm run preview`（预览在 `/anycode/` 下）。

GitHub Actions workflow **`.github/workflows/docs.yml`** 在变更 `docs-site/` 或该工作流时运行 **`npm run build`** 并部署到 Pages。

## Frontmatter convention

Every guide page should start with YAML frontmatter (VitePress uses **`title`** and **`description`** for SEO; the rest are for maintainers):

```yaml
---
title: Short page title
description: One line for meta / search snippets.
summary: One-sentence summary (OpenClaw-style “what this page is”).
read_when:
  - Bullet list of situations where this page is the right read.
---
```

- **`summary`**: keep it one line; mirrors how other projects tag “what this doc covers”.  
- **`read_when`**: 2–4 bullets; helps authors avoid duplicating content in the wrong place.  
- Do not rely on **`summary` / `read_when`** for layout unless a future theme consumes them.

## Information architecture

- **Hub**: [guide/docs-directory.md](guide/docs-directory.md) (EN) and [zh/guide/docs-directory.md](zh/guide/docs-directory.md) (ZH).  
- **Full list**: [guide/hubs.md](guide/hubs.md) and [zh/guide/hubs.md](zh/guide/hubs.md).  
- Sidebar is defined in [`.vitepress/config.ts`](.vitepress/config.ts); keep **EN and ZH sidebars structurally parallel**.

## Link checks

- **`vitepress build`** fails on unresolved **internal** links (except patterns in **`ignoreDeadLinks`**).  
- Links into the repo **`crates/`** tree from Markdown are allowed via **`ignoreDeadLinks`** in `config.ts` because those paths are outside the VitePress root; they are meant for GitHub browsing.  
- Optional: install [lychee](https://github.com/lycheeverse/lychee) or run a small script in CI later; not required for the default workflow.

## Contributing doc edits

1. Match the existing tone and keep EN/zh **symmetric** where the plan calls for it.  
2. After edits, run **`npm run build`** from `docs-site/`.  
3. For Rust cross-links, prefer stable crate paths and keep **`ignoreDeadLinks`** accurate.
