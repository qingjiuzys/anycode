# Fix: test/app-02 runtime Provider error (anycode only)

The Flutter app under `test/app-02/` already exists but has a **runtime bug** (web/mobile):

- Tapping 「开始探索」 on onboarding throws:
  `Could not find the correct Provider<AppProvider> above this OnboardingScreen Widget`
- Root cause: mixed `InspirationService` vs `AppProvider` in `main.dart` vs screens; missing routes.

## Your task

Fix **only** under `test/app-02/` so that:

1. Single consistent state layer (`AppProvider` or one service — not both orphaned).
2. `Provider` wraps the whole app above onboarding and main tabs.
3. Onboarding CTA navigates to the real main UI (no broken named routes).
4. `cd test/app-02 && flutter analyze` — no errors
5. `cd test/app-02 && flutter test` — all pass, including a test that **taps** 「开始探索」 and reaches the today/home shell
6. `test/app-02/README.md` keeps the exact line `GOAL_ACCEPTANCE_OK` and 「增长设计」 section

Do not modify files outside `test/app-02/`.

When finished, final message must include `GOAL_ACCEPTANCE_OK` and confirm analyze/test passed.
