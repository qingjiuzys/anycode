# Official Skills Catalog

Curated skills installable from **Agents → Official catalog** in the workbench.

| ID | Category | Source |
|----|----------|--------|
| web-research | data | anthropics/skills:skills/web-research |
| code-review | quality | anthropics/skills:skills/code-review |
| git-workflow | quality | anthropics/skills:skills/git-workflow |
| office-docx | business | anthropics/skills:skills/docx |
| readonly-db | data | anthropics/skills:skills/readonly-db |

## Install

1. Open `/agents` in the dashboard.
2. Select the **Official catalog** tab.
3. Click **Install** on a skill card.

Skills are copied into your local skills directory and appear under **Installed**.

## Sync script

Regenerate this page from `skill_market.rs`:

```bash
node scripts/sync-skill-catalog-docs.mjs
```
