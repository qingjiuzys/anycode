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
| **`scripts/install.sh`** | macOS / Linux one-command installer; **binary-only by default** |
| **`scripts/install.ps1`** | Windows PowerShell installer; **binary-only by default** |
| **GitHub Releases** | Air-gapped or browser-only download |
| **`cargo install --git`** | You already have Rust and want a specific branch/tag |
| **`git clone` + `cargo build`** | Contributors and feature flags |

## One-line installer (macOS / Linux)

This project’s GitHub repo is **`qingjiuzys/anycode`**:

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

## One-line installer (Windows PowerShell)

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

With explicit repo / version (save then execute):

```powershell
$tmp = Join-Path $env:TEMP "anycode-install.ps1"
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 -OutFile $tmp
& $tmp -Repo qingjiuzys/anycode -Version v0.1.0
```

Or:

```bash
export ANYCODE_GITHUB_REPO="qingjiuzys/anycode"
bash scripts/install.sh
```

Useful flags: `--version v0.1.0` or `latest`; `--bin-dir "$HOME/.local/bin"`; `--dry-run`; `--no-setup` to skip post-install wizard; `--quiet` to reduce download output; `--method auto` to allow source fallback. By default installer runs `anycode setup` after install and shows download progress in interactive terminals. Help:

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

Assets should be named:

- Unix: `anycode-<asset-target>.tar.gz` (archive root contains `anycode`)
- Windows: `anycode-<target>.zip` (archive root contains `anycode.exe`)

For Linux assets, we omit `unknown` from the Rust triple for readability. Typical targets:

- `aarch64-apple-darwin`
- `x86_64-apple-darwin`
- `x86_64-linux-gnu`
- `aarch64-linux-gnu`
- `x86_64-pc-windows-msvc`
- `aarch64-pc-windows-msvc`

## From source

```bash
git clone https://github.com/qingjiuzys/anycode.git
cd anycode
cargo build --release
# run directly: ./target/release/anycode --help
```

Install into `PATH` (recommended, avoids `command not found`):

```bash
./scripts/install.sh --source-dir "$(pwd)"
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

- [Getting started](./getting-started) — `setup` and first task  
- [Config & security](./config-security) — `~/.anycode/config.json`  
