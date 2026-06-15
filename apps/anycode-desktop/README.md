# anyCode Desktop (Tauri)

Desktop shell for Digital Workbench + sidecar services.

App icon source: [`assets/anycode-logo.png`](assets/anycode-logo.png) (brand artwork). Release builds run `scripts/prepare-desktop-icon.py` to crop padding and scale the graphic for Dock visibility, then regenerate `icons/` (`.icns`, `.ico`, platform sizes) from [`assets/anycode-logo-app-icon.png`](assets/anycode-logo-app-icon.png) via `cargo tauri icon`. Requires `python3` + `pillow` (`pip install pillow`).

## Prerequisites

- Rust toolchain
- [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- `cargo-tauri` CLI (`cargo install tauri-cli --version "^2" --locked`) — `scripts/build-desktop-release.sh` installs it if missing
- Built dashboard UI: `../../scripts/build-dashboard-ui.sh`
- `anycode` on PATH (dev) or bundled under `resources/bin/` (release build)

## Development

Terminal 1 — dashboard API:

```bash
anycode dashboard
```

Terminal 2 — desktop shell (opens Workbench at http://127.0.0.1:43180):

```bash
cd apps/anycode-desktop
cargo tauri dev
```

Ensure `resources/bin/anycode` exists (copy from `target/release/anycode`) and `icons/icon.icns` is present before first dev build.

## Sidecar

On launch, the desktop shell **best-effort spawns** `anycode dashboard` and stops all sidecars on quit.

- **Release / `./scripts/build-desktop-release.sh`**: uses bundled `resources/bin/anycode` copied from `target/release/anycode`.
- **Dev (`cargo tauri dev`)**: falls back to `anycode` on `PATH` when the bundled binary is absent.

**Bundled CLI path (release `.app`):**

```bash
/Applications/anyCode.app/Contents/Resources/resources/bin/anycode --help
```

You can symlink that binary to `/usr/local/bin/anycode` if you want terminal access on PATH.

Optional WeChat bridge (same machine):

```bash
ANYCODE_DESKTOP_WECHAT=1 cargo tauri dev
# or for release run:
ANYCODE_DESKTOP_WECHAT=1 open target/release/bundle/macos/anyCode.app
```

Production WeChat is usually handled by LaunchAgent from `anycode channel wechat`.

If the dashboard sidecar fails (e.g. port in use), start it manually: `anycode dashboard`.

## Release build

From repo root:

```bash
chmod +x scripts/build-desktop-release.sh
./scripts/build-desktop-release.sh
```

Output (macOS):

| Artifact | Path |
|----------|------|
| `.app` | `target/release/bundle/macos/anyCode.app` |
| `.dmg` | `target/release/bundle/dmg/anyCode_<version>_aarch64.dmg` |

Tauri shares the repo-root `target/` directory (`apps/anycode-desktop/.cargo/config.toml`) so desktop and CLI builds reuse the same Cargo cache.

### Build-time downloads vs bundled app

| Command | Browser MCP / Chromium | dashboard-ui `npm ci` | Notes |
|---------|------------------------|----------------------|-------|
| `cargo build --release -p anycode` | **No** | Only if `dist/` missing | CLI binary only |
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

## GitHub Release

On tag push (`v*`), [`.github/workflows/desktop-release.yml`](../../.github/workflows/desktop-release.yml) builds the **macOS DMG** and attaches it to the GitHub Release. **Linux/Windows CLI** tarballs are not published on tag; use `cargo install` / build from source, or run [`release-binaries.yml`](../../.github/workflows/release-binaries.yml) manually if needed.

Download: **GitHub → Releases → Assets → `anyCode_*_aarch64.dmg`** (Apple Silicon, CLI bundled inside).

## Optional code signing (CI / release)

Set repository secrets to enable Apple signing in `.github/workflows/desktop-release.yml`:

| Secret | Purpose |
|--------|---------|
| `APPLE_CERTIFICATE_BASE64` | Developer ID `.p12` (base64) |
| `APPLE_CERTIFICATE_PASSWORD` | Export password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: …` |
| `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` | Notarization (Tauri reads env at build) |

Without secrets, CI still uploads **unsigned/ad-hoc** artifacts (same as local `build-desktop-release.sh`).

### CI failure: build and upload locally

If the macOS desktop job fails but you need a DMG on the Release page:

```bash
./scripts/build-desktop-release.sh
gh release upload v0.2.x target/release/bundle/dmg/*.dmg --clobber
```

Ad-hoc DMG: first open may require **System Settings → Privacy & Security** or right-click **Open**. For distribution without Gatekeeper prompts, configure all Apple secrets above for signed + notarized builds.

## Notes

- v0.1 wraps embedded dashboard UI; CLI remains the advanced entry.
- See [docs/comparisons/workbuddy-comparison-2026-06.md](../../docs/comparisons/workbuddy-comparison-2026-06.md) for WorkBuddy parity scope.
