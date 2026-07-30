# User documentation

Canonical user-facing docs for **https://anycode.work/docs/**.

- **English:** `en/guide/`
- **中文:** `zh/guide/`
- **Screenshots:** `assets/screenshots/` (copied to portal as `/docs/assets/…`)

## Edit & preview

1. Edit markdown under `docs/user/`.
2. Add or update screenshots under `docs/user/assets/screenshots/`.
3. Stage into the portal build:

   ```bash
   node scripts/prepare-user-docs.mjs
   ```

4. Preview in account-portal:

   ```bash
   cd crates/account-portal && npm run dev
   ```

   Open http://127.0.0.1:43201/docs

### Screenshot conventions

- PNG, 1440×900 viewport, saved under `assets/screenshots/`
- Reference in markdown: `![caption](/docs/assets/screenshots/name.png)`
- Optional caption on the next line: `*图：说明*` (italic line renders as caption)

Official skills catalog tables are synced from `crates/dashboard/src/skill_market.rs`:

```bash
node scripts/sync-skill-catalog-docs.mjs
```

Maintainer docs (ADRs, ops, architecture) stay under `docs/` at the repo root — not published on the public site.
