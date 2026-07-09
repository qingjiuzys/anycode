# Production launch checklist

## Build & CI

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets`
- [ ] `cargo test --workspace`
- [ ] `cd crates/dashboard-ui && npm test`
- [ ] `cd docs-site && npm ci && npm run build`
- [ ] Tag `v*` → `desktop-release.yml` (DMG + updater artifacts)

## Cloud stack (out of main git)

- [ ] `deploy/account-service/build-push.sh` → K8s roll on anycode.work
- [ ] Migration `007_seed_agnes_model.sql` applied
- [ ] model-gateway healthy at `/v1/models`

## Desktop

- [ ] `ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode-desktop`
- [ ] Optional: `APPLE_*` + `TAURI_SIGNING_PRIVATE_KEY` secrets for signed DMG/updater
- [ ] Install DMG → link cloud → **云端 Auto** chat → usage recorded

## Sites

- [ ] GitHub Pages docs: https://qingjiuzys.github.io/anycode/
- [ ] Portal: https://anycode.work (download CTA + pricing)

## User action (if signing)

Configure repository secrets: `APPLE_CERTIFICATE_BASE64`, `APPLE_SIGNING_IDENTITY`, `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` for Gatekeeper-trusted builds and updater pubkey.

## Deliverables

| Item | Location |
|------|----------|
| macOS DMG | GitHub Releases `anycode-desktop_*.dmg` |
| Updater manifest | `latest.json` on release |
| Daemon | `target/release/anycode-daemon` |
| Scenario eval | [scenario-eval.md](./scenario-eval.md) |
