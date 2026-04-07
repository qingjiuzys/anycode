---
title: Install
description: Install anyCode from GitHub Releases, install.sh, or Cargo.
summary: Prebuilt tarballs, one-line installer, cargo install, and release naming.
read_when:
  - You are installing anyCode on a new machine.
  - You need to choose between binary and building from source.
---

# Install

## Choose a path

| Method | Best for |
|--------|----------|
| **`scripts/install.sh`** | One command; tries Release assets first, then `cargo install --git` if needed |
| **GitHub Releases** | Air-gapped or browser-only download |
| **`cargo install --git`** | You already have Rust and want a specific branch/tag |
| **`git clone` + `cargo build`** | Contributors and feature flags |

## One-line installer

This project’s GitHub repo is **`qingjiuzys/anycode`**:

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

Or:

```bash
export ANYCODE_GITHUB_REPO="qingjiuzys/anycode"
bash scripts/install.sh
```

Useful flags: `--version v0.1.0` or `latest`; `--bin-dir "$HOME/.local/bin"`; `--dry-run`; `--onboard` after install. Help:

```bash
curl -fsSL "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --help
```

## Install v0.1.0 (pinned)

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | \
  bash -s -- --repo qingjiuzys/anycode --version v0.1.0
```

Or install from Cargo with the release tag:

```bash
cargo install --git https://github.com/qingjiuzys/anycode --tag v0.1.0 anycode --force
```

Release page: <https://github.com/qingjiuzys/anycode/releases/tag/v0.1.0>

## Release asset naming

Assets should be named `anycode-<asset-target>.tar.gz` with `anycode` at the **root** of the archive. For Linux assets, we omit `unknown` from the Rust triple for readability. Typical asset targets:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-linux-gnu`
- `aarch64-linux-gnu`

## From source

```bash
git clone https://github.com/qingjiuzys/anycode.git
cd anycode
cargo build --release
# ./target/release/anycode
```

Install into `PATH`:

```bash
cargo install --path crates/cli --force
anycode --help
```

## Local clone only

```bash
./scripts/install.sh --source-dir "$(pwd)" --bin-dir "$HOME/.local/bin"
```

## Optional features

Build with extra capabilities (see root README and [Roadmap](./roadmap)):

```bash
cargo build -p anycode --features tools-mcp
```

## Next

- [Getting started](./getting-started) — `onboard` and first task  
- [Config & security](./config-security) — `~/.anycode/config.json`  
