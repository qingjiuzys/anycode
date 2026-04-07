---
title: Install
description: Install anyCode from GitHub Releases, install.sh, or Cargo.
summary: Recommended install paths for non-technical users first, then advanced options.
read_when:
  - You are installing anyCode on a new machine.
  - You need to choose between binary and building from source.
---

# Install

For users who want anyCode working fast, without building from source.

After this page, you will have:

- `anycode` installed
- a successful `anycode --help` check
- a clear fallback path if install fails

## Recommended path

If you just want to use anyCode, use the one-line installer for your OS.

| OS | Command |
|----|---------|
| macOS / Linux | `curl ... install.sh | bash` |
| Windows | `irm ... install.ps1 | iex` |

## One-line installer (macOS / Linux)

Default repo is **`qingjiuzys/anycode`**:

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

Expected output: installer downloads binary and runs `anycode setup` by default.

## One-line installer (Windows PowerShell)

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

Expected output: PowerShell installer completes and starts setup unless disabled.

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

Expected output: install script uses the repository from env variable.

By default installer:

- installs from prebuilt binary
- shows download progress in interactive terminal
- runs `anycode setup` after install

Useful flags:

- `--version v0.1.0` or `latest`
- `--bin-dir "$HOME/.local/bin"`
- `--no-setup` (skip setup after install)
- `--quiet` (less download output)
- `--method auto` (allow source fallback)

Help:

```bash
curl -fsSL "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --help
```

Next step: pick the flags you need, then rerun install.

## Install v0.1.0 (pinned)

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | \
  bash -s -- --repo qingjiuzys/anycode --version v0.1.0
```

Expected output: installs pinned version `v0.1.0`.

Or install from Cargo with the release tag:

```bash
cargo install --git https://github.com/qingjiuzys/anycode --tag v0.1.0 anycode --force
```

Release page: <https://github.com/qingjiuzys/anycode/releases/tag/v0.1.0>

## Verify install

```bash
anycode --help
anycode setup
```

Expected output: `--help` shows command list; `setup` opens onboarding flow.

If you see `command not found`, check PATH notes from the installer output and retry in a new shell.

## Source build (advanced)

For contributors or custom builds:

```bash
git clone https://github.com/qingjiuzys/anycode.git
cd anycode
cargo build --release
# run directly: ./target/release/anycode --help
```

Expected output: release build succeeds and binary is available under `target/release`.

Install into `PATH` (recommended, avoids `command not found`):

```bash
./scripts/install.sh --source-dir "$(pwd)"
anycode --help
```

Next step: run `anycode setup`.

## Local clone only

```bash
./scripts/install.sh --source-dir "$(pwd)" --bin-dir "$HOME/.local/bin"
```

## Optional features

Build with extra capabilities:

```bash
cargo build -p anycode --features tools-mcp
```

Expected output: binary compiled with MCP capability.

## Next

- [Getting started](./getting-started) — `setup` and first task  
- [Config & security](./config-security) — `~/.anycode/config.json`  
