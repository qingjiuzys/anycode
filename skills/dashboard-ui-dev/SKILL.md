---
name: dashboard-ui-dev
description: Develop and verify the anyCode dashboard React UI (Vite + TanStack).
---

# dashboard-ui-dev

Work in `crates/dashboard-ui/`:

- Reuse components under `src/components/ui/` before adding new primitives.
- i18n: update both `src/i18n/en.ts` and `src/i18n/zh.ts`.
- API client: `src/api/client/`; types in `src/api/types/`.

After UI changes, run TypeScript/build checks if available in the project scripts, and exercise the page in the running dashboard when the user has `anycode dashboard` up.

Match existing Tailwind utility patterns and `dw-*` CSS classes in `src/index.css`.
