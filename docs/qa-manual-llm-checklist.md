# Real LLM Manual Test Checklist

> Automated coverage: Playwright `ui-interactions.spec.ts` (47 tests) + `fixture_api` integration tests.
> This checklist covers paths that require a live LLM provider.

Prerequisites:
- `~/.anycode/config.json` has a working provider/model/API key
- At least one scanned project
- Run: `anycode dashboard --open`

| # | Path | Steps | Pass |
|---|------|-------|------|
| 1 | Home `/` | Select project → enter short prompt → Send → streaming reply appears | ☐ |
| 2 | Conversations | New session → follow-up message → transcript updates | ☐ |
| 3 | Tool approval | Trigger Bash or sensitive tool → SecurityApprovalInbox → Allow once → session continues | ☐ |
| 4 | AskUserQuestion | Trigger question tool → AskUserQuestionInbox → submit answer → agent continues | ☐ |
| 5 | Text upload | Attach `.txt` in composer → send → agent references file content | ☐ |
| 6 | Cancel | Running session → Cancel → status becomes cancelled | ☐ |
| 7 | Setup | `/setup?review=1` → LLM test → memory save → complete | ☐ |
| 8 | Project gate | Project detail → execute gate → result shown | ☐ |
| 9 | Cron | Automations → create cron → observe run row / retry if failed | ☐ |

CLI parallel:
- `anycode status` shows provider
- `anycode setup --help` exits 0
- TTY `anycode` REPL accepts one message with real LLM (optional)
