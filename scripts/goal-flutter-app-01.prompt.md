# Fuzzy goal: production Flutter MVP (test/app-01)

Build a shippable Flutter app under `test/app-01/` only. Product: one-line daily journal for Chinese users (streak, history, share copy).

## Acceptance (verify before claiming done)

1. `cd test/app-01 && flutter analyze` — no errors
2. `cd test/app-01 && flutter test` — all pass
3. `test/app-01/README.md` contains the exact line `GOAL_ACCEPTANCE_OK`
4. At least 3 screens (tabs or routes), Material 3, light/dark theme
5. Real 简体中文 UI copy (not placeholder English demo)

## Constraints

- Only write under `test/app-01/` (create dir if needed)
- Run `flutter create` when no project exists
- Minimal dependencies

When truly finished, your **final assistant message** must include `GOAL_ACCEPTANCE_OK` and briefly confirm analyze/test passed.
