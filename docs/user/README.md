# User documentation

Canonical user-facing docs for **https://anycode.work/docs/**.

- **English:** `en/guide/`
- **中文:** `zh/guide/`

## Edit & preview

1. Edit markdown under `docs/user/`.
2. Stage into the portal build:

   ```bash
   node scripts/prepare-user-docs.mjs
   ```

3. Preview in account-portal:

   ```bash
   cd crates/account-portal && npm run dev
   ```

   Open http://127.0.0.1:43201/docs

Official skills catalog tables are synced from `crates/dashboard/src/skill_market.rs`:

```bash
node scripts/sync-skill-catalog-docs.mjs
```

Maintainer docs (ADRs, ops, architecture) stay under `docs/` at the repo root — not published on the public site.
