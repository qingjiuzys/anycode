# Fuzzy goal: growth-oriented Flutter MVP (test/app-02)

Build a **shippable, user-growth-oriented** Flutter app under `test/app-02/` only.

## Product (must implement)

**「每日灵感卡片」** — a lightweight app for Chinese users designed to **attract and retain** users:

- **Core loop**: open app → see today's curated 简体中文 inspiration card (quote + short action tip).
- **Retention**: daily streak counter ("连续打开天数"); local persistence.
- **Growth / viral**: one-tap **分享** (use `share_plus`) with a polished share template string (include app name + today's card text + invite line).
- **Engagement**: **收藏** favorite cards; **历史** browse past days/cards (at least 7 seeded sample cards in code or local store).
- **Onboarding**: first-launch welcome screen (1 page) explaining value in 简体中文, then enter main app.

## UX requirements

1. At least **4 screens/routes**: 欢迎(onboarding), 今日, 收藏, 我的 (settings/theme/about).
2. **Material 3**, light + dark theme toggle in 我的.
3. All user-visible strings in **简体中文** (no English placeholder demo UI).
4. Visually appealing: use Material 3 color scheme, cards, typography hierarchy — not a bare scaffold.

## Acceptance (verify before claiming done)

1. `cd test/app-02 && flutter analyze` — no errors
2. `cd test/app-02 && flutter test` — all pass
3. `test/app-02/README.md` contains the exact line `GOAL_ACCEPTANCE_OK`
4. README includes a short **「增长设计」** section (3–5 bullets: 分享、连续打开、收藏、 onboarding 等)
5. Share + streak + favorites + onboarding are wired and testable (widget test at minimum)
6. `widget_test` must **tap** the onboarding CTA (e.g. 「开始探索」) and assert the main shell loads — root `Provider` must wrap `MaterialApp` (no `ProviderNotFound` at runtime on web/mobile)

## Constraints

- Only write under `test/app-02/` (create dir if needed)
- Run `flutter create` when no project exists
- Minimal dependencies (`shared_preferences`, `intl`, `share_plus` ok; avoid heavy packages)
- Do **not** modify `test/app-01/` or other paths

When truly finished, your **final assistant message** must include `GOAL_ACCEPTANCE_OK` and briefly confirm analyze/test passed.
