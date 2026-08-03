# macOS desktop release (local)

GitHub Actions desktop builds are **optional / manual only** (slow). Ship DMG from your Mac.

## One-time setup

1. **Developer ID Application** cert in Keychain (not Apple Development).
2. Copy `scripts/release.env.example` → `~/.anycode/release.env` and fill:
   - `APPLE_SIGNING_IDENTITY`
   - `APPLE_TEAM_ID`
   - `APPLE_ID`
   - `APPLE_PASSWORD` (app-specific password)

## Ship a version

```bash
# 1. Bump workspace version in Cargo.toml, then:
./scripts/sync-workspace-version.sh

# 2. Build signed + notarized DMG and stage for portal
chmod +x scripts/release-desktop-local.sh
./scripts/release-desktop-local.sh                 # Apple Silicon (host)
./scripts/release-desktop-local.sh --arch x86_64   # Intel (cross from Apple Silicon)

# Windows (on a Windows host after build-desktop-release.sh):
./scripts/stage-desktop-windows.sh
```

Artifacts land in `crates/account-portal/public/downloads/`:

| File | Purpose |
|------|---------|
| `anyCode_<version>_aarch64.dmg` | macOS Apple Silicon |
| `anyCode_<version>_x86_64.dmg` | macOS Intel |
| `anyCode_<version>_x64.msi` / `.exe` | Windows (when staged) |
| `anyCode_latest_<arch>.dmg` | Stable “latest” link per arch |
| `latest.json` | Latest metadata (+ `platforms` map) |
| `releases.json` | Multi-version / multi-platform catalog |
| `SHA256SUMS.txt` | Checksums for all staged files |

Public URLs (after deploy):

- https://anycode.work/downloads/anyCode_latest_aarch64.dmg
- https://anycode.work/downloads/anyCode_latest_x86_64.dmg
- https://anycode.work/downloads/releases.json

## Deploy to anycode.work (recommended)

DMG is **baked into the account+portal Docker image** — deploy the image and downloads go live automatically.

```bash
# macOS: signed DMG → portal public/downloads → docker build → ACR push
./scripts/build-account-image.sh
# or: TAG=0.2.4 ./scripts/build-account-image.sh

kubectl set image deployment/anycode-account \
  anycode-account=registry.cn-zhangjiakou.aliyuncs.com/818cloud/anycode:0.2.4
kubectl rollout status deployment/anycode-account
```

No separate rsync/upload step on deploy. Portal Vite build copies `public/downloads/` into the image at `/app/portal/downloads/`.

If DMG is already staged and you only need to rebuild the image:

```bash
./scripts/build-account-image.sh --skip-dmg
```

## Verify on a clean Mac

```bash
spctl -a -vv -t install target/release/bundle/macos/anyCode.app
# expect: accepted / Notarized Developer ID
```

Signed/notarized releases **omit bundled Chromium** (Playwright cannot be re-signed for notarization). Browser MCP installs Chromium on first use.

`createUpdaterArtifacts` is off until `TAURI_SIGNING_PRIVATE_KEY` is configured; updates ship via new DMG in the portal image.

## GitHub Releases (optional)

If you still want a GitHub asset mirror:

```bash
gh release create v0.2.4 crates/account-portal/public/downloads/anyCode_0.2.4_aarch64.dmg --title v0.2.4
```

Primary download channel remains **anycode.work**.
