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
| `.app` | `apps/anycode-desktop/target/release/bundle/macos/anyCode.app` |
| `.dmg` | `apps/anycode-desktop/target/release/bundle/dmg/anyCode_<version>_aarch64.dmg` |

The release bundle includes **Playwright MCP + Chromium** under `resources/browser/` (no manual `npx playwright install`). Enable in Workbench → **Settings → Notifications → Browser connector**, then start a new conversation.

## GitHub Release

On tag push (`v*`), [`.github/workflows/desktop-release.yml`](../../.github/workflows/desktop-release.yml) builds the DMG and attaches it to the GitHub Release (alongside CLI tarballs from `release-binaries.yml`).

Download: **GitHub → Releases → Assets → `anyCode_*_aarch64.dmg`** (Apple Silicon).

## Optional code signing (CI / release)

Set repository secrets to enable Apple signing in `.github/workflows/desktop-release.yml`:

| Secret | Purpose |
|--------|---------|
| `APPLE_CERTIFICATE_BASE64` | Developer ID `.p12` (base64) |
| `APPLE_CERTIFICATE_PASSWORD` | Export password |
| `APPLE_SIGNING_IDENTITY` | e.g. `Developer ID Application: …` |
| `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` | Notarization (Tauri reads env at build) |

Without secrets, CI still uploads **unsigned** artifacts (same as local `build-desktop-release.sh`).

## Notes

- v0.1 wraps embedded dashboard UI; CLI remains the advanced entry.
- See [docs/workbuddy-comparison-2026-06.md](../../docs/workbuddy-comparison-2026-06.md) for WorkBuddy parity scope.
