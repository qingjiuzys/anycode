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

| OS | Recommended install |
|----|---------------------|
| **macOS** | Download **`anyCode_<version>_aarch64.dmg`** from [GitHub Releases](https://github.com/qingjiuzys/anycode/releases) — bundles CLI + Workbench + native STT/OCR. No separate macOS CLI tarball. |
| **Linux** | `curl ... install.sh \| bash` |
| **Windows** | `irm ... install.ps1 \| iex` |

### macOS desktop app (recommended)

1. Open [Releases](https://github.com/qingjiuzys/anycode/releases) and download **`anyCode_<version>_aarch64.dmg`**.
2. Open the DMG and drag **anyCode** to Applications.
3. Launch **anyCode** — it starts the bundled CLI sidecar (`anycode dashboard`) and opens Workbench.

The CLI is **inside the app bundle** (not a separate Release asset on macOS):

```bash
/Applications/anyCode.app/Contents/Resources/resources/bin/anycode --help
```

Use `install.sh` on macOS only for headless servers or when you want `anycode` on PATH without the desktop app.

## One-line installer (Linux)

Default repo is **`qingjiuzys/anycode`**:

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --repo qingjiuzys/anycode
```

Expected output: installer downloads binary and runs `anycode setup` by default (`setup` includes a memory / embedding step on interactive terminals; see [Memory notes](./memory)).

## One-line installer (Windows PowerShell)

```powershell
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 | iex
```

Expected output: PowerShell installer completes and starts setup unless disabled.

With explicit repo / version (save then execute):

```powershell
$tmp = Join-Path $env:TEMP "anycode-install.ps1"
irm https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.ps1 -OutFile $tmp
& $tmp -Repo qingjiuzys/anycode -Version latest
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

- `--version latest` or `--version v0.2.2` (pin a release tag)
- `--bin-dir "$HOME/.local/bin"`
- `--no-setup` (skip setup after install)
- `--quiet` (less download output)
- `--method auto` (allow source fallback)

Help:

```bash
curl -fsSL "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | bash -s -- --help
```

Next step: pick the flags you need, then rerun install.

## Install a pinned release

Default one-liner installs the [latest GitHub Release](https://github.com/qingjiuzys/anycode/releases/latest). To pin the current workspace version:

```bash
curl -fsSL --proto '=https' --tlsv1.2 \
  "https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh" | \
  bash -s -- --repo qingjiuzys/anycode --version v0.2.2
```

Expected output: installs pinned version `v0.2.2`.

Or install from Cargo with the release tag:

```bash
cargo install --git https://github.com/qingjiuzys/anycode --tag v0.2.2 anycode --force
```

Release page: <https://github.com/qingjiuzys/anycode/releases>

**Release assets by platform:**

| Platform | GitHub Release asset |
|----------|----------------------|
| macOS (Apple Silicon) | `anyCode_<version>_aarch64.dmg` (CLI bundled inside) |
| Linux x86_64 / arm64 | `anycode-<target>.tar.gz` |
| Windows x86_64 / arm64 | `anycode-<target>.zip` |

Build the macOS desktop locally: `./scripts/build-desktop-release.sh`.

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
