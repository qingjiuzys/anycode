---
title: Releases & feature flags
description: Versioning expectations, GitHub Releases, and anycode enable/disable flags.
summary: Where updates ship; how to toggle experimental runtime features from the CLI.
read_when:
  - You publish or consume anyCode builds.
  - You want a single entry point for experimental toggles.
---

# Releases & feature flags

## Versioning

- **Library / CLI version** follows the workspace `version` in the root `Cargo.toml`.
- **GitHub Releases**: tag and attach `anycode` binaries for your platform when distributing outside `cargo install`.
- **Docs site** (VitePress under `docs-site/`): deploy to GitHub Pages with `VITEPRESS_BASE=/your-repo/` when using project pages.

## Runtime feature flags {#runtime-feature-flags}

Use the CLI as the **single toggle surface**:

```bash
anycode enable skills
anycode disable workflows
anycode status
```

Recognized names (see `anycode_core::FeatureFlag`):

| Flag | `enable` / `disable` name |
|------|---------------------------|
| Skills scanning in CLI | `skills` |
| Workflow helpers | `workflows` or `workflow` |
| Goal-oriented mode affordances | `goal-mode` or `goal` |
| Channel-oriented defaults | `channel-mode` or `channel` |
| Experimental approval path | `approval-v2` or `approval` |
| Context compaction affordances | `context-compression` or `compact` |
| Workspace profile overlays | `workspace-profiles` or `workspace` |

## Related

- [CLI overview](./cli) — global flags  
- [Routing](./routing) — `model_routes` and workspace overlays  
