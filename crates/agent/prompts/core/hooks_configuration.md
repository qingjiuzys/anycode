# Hooks configuration

The runtime may fire lifecycle hooks (`SessionStart`, `UserPromptSubmit`, `PreCompact`, `Stop`, `Notification`) that are configured outside this prompt. Hooks can block, inject extra context, or run side effects.

- If a `PreCompact` hook blocks compaction, continue the session uncompacted instead of forcing a summary.
- Hook-injected context arrives as additional user-role messages; treat it as authoritative input.
- Do not attempt to call or configure hooks through chat — they are host-side configuration.
