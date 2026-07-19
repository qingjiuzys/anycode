# Production launch checklist

## Build & CI

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets`
- [ ] `cargo test --workspace`
- [ ] `cd crates/dashboard-ui && npm test`
- [ ] `cd crates/account-portal && npm ci && npm run build`
- [ ] `./scripts/build-account-image.sh` → ACR image with portal + DMG baked in
- [ ] `kubectl rollout` on anycode.work

## Cloud stack (out of main git)

- [ ] `deploy/account-service/build-push.sh` → K8s roll on anycode.work
- [ ] Migration `007_seed_agnes_model.sql` applied
- [ ] model-gateway healthy at `/v1/models`

## Desktop

- [ ] `~/.anycode/release.env` configured (see `scripts/release.env.example`)
- [ ] `./scripts/release-desktop-local.sh` → trusted DMG staged for portal
- [ ] Install DMG → link cloud → **云端 Auto** chat → usage recorded

## Sites

- [ ] Docs site live: https://anycode.work/docs/
- [ ] Portal: https://anycode.work (download CTA + pricing)

## User action (signing)

Copy `scripts/release.env.example` to `~/.anycode/release.env` with Apple Developer ID + app-specific password. See [desktop-release-local.md](./desktop-release-local.md).

## Deliverables

| Item | Location |
|------|----------|
| macOS DMG | Baked in ACR image → `anycode.work/downloads/anyCode_latest_aarch64.dmg` |
| Updater manifest | `latest.json` on release |
| Daemon | `target/release/anycode-daemon` |
| Scenario eval | [scenario-eval.md](./scenario-eval.md) |
