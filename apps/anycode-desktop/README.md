# anyCode Desktop (Tauri)

Desktop shell for Digital Workbench (dashboard runs **in-process**, no CLI sidecar).

App icon source: [`assets/anycode-logo.png`](assets/anycode-logo.png) (brand artwork). Release builds run `scripts/prepare-desktop-icon.py` to crop white margins and emit a **full-bleed opaque** PNG (no transparent ring — macOS applies the Dock squircle), then regenerate `icons/` (`.icns`, `.ico`, platform sizes) from [`assets/anycode-logo-app-icon.png`](assets/anycode-logo-app-icon.png) via `cargo tauri icon`. Requires `python3` + `pillow` (`pip install pillow`). Force refresh: `ANYCODE_DESKTOP_ICON_FORCE=1`.

## Prerequisites

- Rust toolchain
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- `cargo-tauri` CLI (`cargo install tauri-cli --version "^2" --locked`) — `scripts/build-desktop-release.sh` installs it if missing
- Built dashboard UI: `../../scripts/build-dashboard-ui.sh`
- Release builds embed the dashboard in-process (no separate `anycode` CLI)

## Development

**Fast iteration (recommended):**

```bash
./scripts/sync-desktop-dev.sh              # UI-only → /Applications/anyCode.app (~15s)
./scripts/sync-desktop-dev.sh --rust       # UI + Rust shell (release-local, ~30–40s incremental)
```

Avoid `cargo build --release -p anycode-desktop` during daily work — release enables LTO and takes ~5min. Use `./scripts/build-desktop-local.sh` only when you need a shipping DMG.

```bash
cd apps/anycode-desktop
cargo tauri dev
```

The dev shell starts the in-process Workbench API at `127.0.0.1:43180` (API only). The UI loads from the app bundle, not from that URL in a browser.

## Embedded dashboard

On launch, the app starts **anycode-dashboard** in-process at `127.0.0.1:43180` for `/api/*`, SSE, and WebSockets (see `src/dashboard_backend.rs`). The Workbench UI is loaded from bundled `dashboard-ui` assets inside the Tauri webview — **do not** open `http://127.0.0.1:43180/` in Chrome/Safari (that URL is API-only). No `anycode dashboard` subprocess.

Optional WeChat bridge on the same machine uses **`anycode-daemon wechat-bridge`** (not bundled in the app by default):

```bash
ANYCODE_DESKTOP_WECHAT=1 cargo tauri dev
```

Headless channels/cron on servers: install `anycode-daemon` separately.

## Release build (local, signed DMG)

Primary distribution: **local build** → upload to **https://anycode.work/downloads/**.

See [docs/ops/desktop-release-local.md](../../docs/ops/desktop-release-local.md).

```bash
# ~/.anycode/release.env — copy from scripts/release.env.example
./scripts/release-desktop-local.sh
```

Quick unsigned dev DMG (no notarization):

```bash
./scripts/build-desktop-release.sh
```

Output (macOS):

| Artifact | Path |
|----------|------|
| `.app` | `target/release/bundle/macos/anyCode.app` |
| `.dmg` | `target/release/bundle/dmg/anyCode_<version>_aarch64.dmg` |

Tauri shares the repo-root `target/` directory (`apps/anycode-desktop/.cargo/config.toml`).

### Build-time downloads vs bundled app

| Command | Browser MCP / Chromium | dashboard-ui `npm ci` | Notes |
|---------|------------------------|----------------------|-------|
| `cargo build --release -p anycode-desktop` | **No** | Only if `dist/` missing | Desktop app |
| `./scripts/build-desktop-release.sh` | **Yes** (first time or lockfile change) | **Yes** (first time or lockfile change) | Stages into `resources/browser/` then Tauri bundles it into `.app` / `.dmg` |

End users who install the DMG **do not** download Playwright at runtime.

**Repeat local desktop builds** reuse caches when lockfiles and platform are unchanged:

| Cache | Location | Force refresh |
|-------|----------|---------------|
| dashboard-ui npm | `crates/dashboard-ui/.npm-fingerprint` | `ANYCODE_DASHBOARD_UI_FORCE=1` |
| browser MCP + Chromium | `resources/browser/.bundle-fingerprint` | `ANYCODE_BROWSER_MCP_FORCE=1` |
| desktop icons | `icons/.icon-fingerprint` | `ANYCODE_DESKTOP_ICON_FORCE=1` |
| apple-media Swift | mtime vs `resources/bin/anycode-apple-media` | `ANYCODE_APPLE_MEDIA_FORCE=1` |
| staged resources | `resources/.stage-fingerprint` | `ANYCODE_DESKTOP_STAGE_FORCE=1` |
| dashboard-ui vite | `crates/dashboard-ui/.dist-fingerprint` | `ANYCODE_DASHBOARD_UI_FORCE=1` |

**Faster local iterative DMG** (same bundle contents, desktop shell compiles without LTO):

```bash
ANYCODE_DESKTOP_LOCAL_RELEASE=1 ./scripts/build-desktop-release.sh
```

Use plain `./scripts/build-desktop-release.sh` (profile `release` + LTO) for shipping builds.

`build-desktop-release.sh` prints per-step timings and total seconds. Typical repeat build (no lockfile changes): no `npm ci`, no Playwright download; mostly incremental Rust/Swift + DMG packaging.

Install `cargo-tauri` once to avoid in-script `cargo install`:

```bash
cargo install tauri-cli --version "^2" --locked
```

If dashboard-ui is already built, skip the UI npm step during Rust release builds with `ANYCODE_SKIP_DASHBOARD_UI_BUILD=1` (see `crates/dashboard/build.rs`).

Other models (Whisper, FastEmbed, Piper voices) are **not** bundled at build time; they download on first use under `~/.anycode` or `~/.cache`.

## GitHub Release (optional)

CI desktop job is **manual only** (`workflow_dispatch`). Prefer local signed builds.

If needed: Actions → Desktop release → Run workflow, or upload a local DMG:

```bash
gh release upload v0.2.x crates/account-portal/public/downloads/anyCode_*.dmg --clobber
```

Primary download: **https://anycode.work/downloads/anyCode_latest_aarch64.dmg**

## Code signing (local)

Copy `scripts/release.env.example` → `~/.anycode/release.env`:

| Variable | Purpose |
|----------|---------|
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: …` |
| `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` | Notarization |

`./scripts/release-desktop-local.sh` runs `build-desktop-release.sh` with these env vars and verifies Gatekeeper acceptance.

## Notes

- v0.2+ embeds dashboard in-process; use `anycode-daemon` for headless channels on servers.
- See [docs/comparisons/workbuddy-comparison-2026-06.md](../../docs/comparisons/workbuddy-comparison-2026-06.md) for WorkBuddy parity scope.
