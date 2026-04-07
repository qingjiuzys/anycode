#!/usr/bin/env bash
# anyCode installer (macOS / Linux) — OpenClaw-style one-liner friendly.
#
# Canonical repo: qingjiuzys/anycode
# One-liner:
#   curl -fsSL --proto '=https' --tlsv1.2 \
#     https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh | bash -s -- --repo qingjiuzys/anycode
#
# After you publish releases with asset names: anycode-<asset-target>.tar.gz (binary "anycode" at archive root),
# where linux asset-target omits "-unknown-" for readability (e.g. x86_64-linux-gnu),
# the script downloads from https://github.com/qingjiuzys/anycode/releases
#
# Env:
#   ANYCODE_GITHUB_REPO   default for --repo (e.g. myorg/anycode)
#   ANYCODE_VERSION       tag or "latest" (default: latest)
#   ANYCODE_INSTALL_BIN   directory for the binary (default: first writable PATH dir, else $HOME/.local/bin)
#   ANYCODE_NO_ONBOARD    if 1, do not suggest running onboard at the end
#
set -euo pipefail

readonly PROG="${0##*/}"

info() { printf '%s\n' "$*"; }
warn() { printf '[anycode-install] %s\n' "$*" >&2; }
die() { warn "$*"; exit 1; }

DOWNLOADER=""
detect_downloader() {
  if command -v curl >/dev/null 2>&1; then
    DOWNLOADER=curl
    return 0
  fi
  if command -v wget >/dev/null 2>&1; then
    DOWNLOADER=wget
    return 0
  fi
  die "Need curl or wget."
}

download() {
  local url="$1" out="$2"
  [[ -n "$DOWNLOADER" ]] || detect_downloader
  if [[ "$DOWNLOADER" == curl ]]; then
    curl -fsSL --proto '=https' --tlsv1.2 --retry 3 --retry-delay 1 -o "$out" "$url"
  else
    wget -q --https-only --secure-protocol=TLSv1_2 --tries=3 -O "$out" "$url"
  fi
}

detect_target_triple() {
  local os arch
  os="$(uname -s 2>/dev/null || true)"
  arch="$(uname -m 2>/dev/null || true)"
  case "$os/$arch" in
    Darwin/arm64|Darwin/aarch64) echo "aarch64-apple-darwin" ;;
    Darwin/x86_64) echo "x86_64-apple-darwin" ;;
    Linux/x86_64|Linux/amd64) echo "x86_64-unknown-linux-gnu" ;;
    Linux/aarch64|Linux/arm64) echo "aarch64-unknown-linux-gnu" ;;
    *) die "Unsupported OS/ARCH: ${os}/${arch} (contributions welcome)." ;;
  esac
}

resolve_latest_tag() {
  local repo="$1"
  local final
  final="$(curl -fsSL -L -o /dev/null -w '%{url_effective}' --proto '=https' --tlsv1.2 \
    "https://github.com/${repo}/releases/latest")" || return 1
  [[ "$final" == *"/tag/"* ]] || return 1
  printf '%s\n' "${final##*/}"
}

normalize_version() {
  local v="$1"
  if [[ "$v" == "latest" ]]; then
    printf '%s\n' "latest"
    return
  fi
  if [[ "$v" == v* ]]; then
    printf '%s\n' "$v"
  else
    printf '%s\n' "v${v}"
  fi
}

usage() {
  cat <<'EOF'
Usage: install.sh [options]

  --repo OWNER/REPO     GitHub repository (default: $ANYCODE_GITHUB_REPO; canonical: qingjiuzys/anycode)
  --version TAG         Release tag: v0.1.0, 0.1.0, or latest (default: latest or $ANYCODE_VERSION)
  --bin-dir DIR         Install directory for `anycode` (default: $ANYCODE_INSTALL_BIN, else first writable PATH dir, else $HOME/.local/bin)
  --method MODE         auto | binary | source (default: auto)
                          auto: try GitHub Release tarball, then cargo install --git
  --source-dir PATH     Use `cargo install --path PATH/crates/cli` instead of git (for local dev clone)
  --dry-run             Print actions only
  --onboard             After install, run `anycode onboard` (interactive)
  -h, --help            This help

Release asset layout (for --method binary):
  https://github.com/qingjiuzys/anycode/releases/download/<tag>/anycode-<asset-target>.tar.gz
  linux asset-target omits "-unknown-" (x86_64-linux-gnu / aarch64-linux-gnu)
  Archive must contain executable `anycode` at the top level.

Examples:
  ANYCODE_GITHUB_REPO=qingjiuzys/anycode bash install.sh
  curl -fsSL https://raw.githubusercontent.com/qingjiuzys/anycode/main/scripts/install.sh | bash -s -- --repo qingjiuzys/anycode
  ./scripts/install.sh --method source --source-dir "$PWD"
EOF
}

REPO="${ANYCODE_GITHUB_REPO:-}"
VERSION_INPUT="${ANYCODE_VERSION:-latest}"
BIN_DIR="${ANYCODE_INSTALL_BIN:-}"
BIN_DIR_EXPLICIT=0
[[ -n "${ANYCODE_INSTALL_BIN:-}" ]] && BIN_DIR_EXPLICIT=1
METHOD=auto
SOURCE_DIR=""
DRY_RUN=0
RUN_ONBOARD=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --repo) REPO="${2:-}"; shift 2 ;;
    --version) VERSION_INPUT="${2:-}"; shift 2 ;;
    --bin-dir) BIN_DIR="${2:-}"; BIN_DIR_EXPLICIT=1; shift 2 ;;
    --method) METHOD="${2:-}"; shift 2 ;;
    --source-dir) SOURCE_DIR="${2:-}"; shift 2 ;;
    --dry-run) DRY_RUN=1; shift ;;
    --onboard) RUN_ONBOARD=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "Unknown option: $1 (try --help)" ;;
  esac
done

[[ -n "$REPO" || -n "$SOURCE_DIR" ]] || die "Set --repo OWNER/REPO or ANYCODE_GITHUB_REPO, or use --source-dir for local install."

case "$METHOD" in
  auto|binary|source) ;;
  *) die "--method must be auto, binary, or source" ;;
esac

run() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '[dry-run]'
    printf ' %q' "$@"
    printf '\n'
    return 0
  fi
  "$@"
}

choose_default_bin_dir() {
  [[ "$BIN_DIR_EXPLICIT" -eq 1 ]] && return 0
  local d parent
  # Prefer a writable directory that is already on PATH.
  for d in "${HOME}/.local/bin" "/opt/homebrew/bin" "/usr/local/bin"; do
    case ":${PATH:-}:" in
      *":${d}:"*)
        if [[ -d "$d" ]]; then
          [[ -w "$d" ]] && {
            BIN_DIR="$d"
            return 0
          }
        else
          parent="$(dirname "$d")"
          [[ -d "$parent" && -w "$parent" ]] && {
            BIN_DIR="$d"
            return 0
          }
        fi
        ;;
    esac
  done
  BIN_DIR="${HOME}/.local/bin"
}

ensure_bin_dir() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    info "[dry-run] mkdir -p $(printf '%q' "$BIN_DIR")"
    return 0
  fi
  mkdir -p "$BIN_DIR"
}

install_from_binary() {
  local repo="$1" version_arg="$2" target="$3"
  local tag url tmpdir tf dest asset_target compat_target
  if [[ "$version_arg" == "latest" ]]; then
    tag="$(resolve_latest_tag "$repo")" || return 1
  else
    tag="$(normalize_version "$version_arg")"
  fi
  tmpdir="$(mktemp -d "${TMPDIR:-/tmp}/anycode-install.XXXXXX")"
  tf="${tmpdir}/anycode.tgz"
  asset_target="${target/-unknown-/-}"
  compat_target="${target}"

  url="https://github.com/${repo}/releases/download/${tag}/anycode-${asset_target}.tar.gz"
  warn "Downloading: $url"
  if [[ "$DRY_RUN" -eq 1 ]]; then
    info "[dry-run] download $url -> $tf"
    if [[ "$asset_target" != "$compat_target" ]]; then
      info "[dry-run] fallback if missing: https://github.com/${repo}/releases/download/${tag}/anycode-${compat_target}.tar.gz"
    fi
    rm -rf "$tmpdir"
    return 0
  fi
  if ! download "$url" "$tf"; then
    if [[ "$asset_target" != "$compat_target" ]]; then
      url="https://github.com/${repo}/releases/download/${tag}/anycode-${compat_target}.tar.gz"
      warn "Primary asset missing; trying legacy name: $url"
      if ! download "$url" "$tf"; then
        rm -rf "$tmpdir"
        return 1
      fi
    else
      rm -rf "$tmpdir"
      return 1
    fi
  fi
  tar -xzf "$tf" -C "$tmpdir"
  [[ -f "$tmpdir/anycode" ]] || {
    rm -rf "$tmpdir"
    die "Archive missing top-level 'anycode' binary."
  }
  dest="${BIN_DIR}/anycode"
  run install -m 0755 "$tmpdir/anycode" "$dest"
  rm -rf "$tmpdir"
  info "Installed: $dest"
  return 0
}

# cargo install --root R installs to R/bin/name — only valid when BIN_DIR == R/bin.
cargo_install_copy() {
  local -a cargo_args=("$@")
  if [[ "$DRY_RUN" -eq 1 ]]; then
    if [[ "$(basename "$BIN_DIR")" == "bin" ]]; then
      local r
      r="$(dirname "$BIN_DIR")"
      [[ "$r" == "." ]] && r="$HOME/.local"
      info "[dry-run] cargo ${cargo_args[*]} --root $(printf '%q' "$r") --force"
    else
      info "[dry-run] cargo ${cargo_args[*]} --root <tmp> --force; install to $(printf '%q' "${BIN_DIR}/anycode")"
    fi
    return 0
  fi
  if [[ "$(basename "$BIN_DIR")" == "bin" ]]; then
    local root
    root="$(dirname "$BIN_DIR")"
    [[ "$root" == "." ]] && root="$HOME/.local"
    run cargo "${cargo_args[@]}" --root "$root" --force
    info "Installed: ${BIN_DIR}/anycode"
  else
    local t
    t="$(mktemp -d "${TMPDIR:-/tmp}/anycode-cargo.XXXXXX")"
    run cargo "${cargo_args[@]}" --root "$t" --force
    run install -m 0755 "${t}/bin/anycode" "${BIN_DIR}/anycode"
    rm -rf "$t"
    info "Installed: ${BIN_DIR}/anycode"
  fi
}

install_from_git() {
  local repo="$1"
  local url="https://github.com/${repo}.git"
  warn "Installing from source via cargo (needs Rust toolchain)..."
  command -v cargo >/dev/null 2>&1 || die "cargo not found. Install Rust: https://rustup.rs then retry."
  cargo_install_copy install --locked --git "$url" --package anycode
}

install_from_source_dir() {
  local dir="$1"
  local cli="${dir}/crates/cli"
  [[ -f "${cli}/Cargo.toml" ]] || die "--source-dir must point to repo root containing crates/cli (got $dir)"
  command -v cargo >/dev/null 2>&1 || die "cargo not found. Install Rust: https://rustup.rs"
  warn "cargo install --path $(printf '%q' "$cli") -> $(printf '%q' "${BIN_DIR}/anycode")"
  cargo_install_copy install --locked --path "$cli"
}

main() {
  detect_downloader
  choose_default_bin_dir
  [[ -n "$BIN_DIR" ]] || BIN_DIR="${HOME}/.local/bin"
  ensure_bin_dir
  local target
  target="$(detect_target_triple)"

  if [[ -n "$SOURCE_DIR" ]]; then
    install_from_source_dir "$(cd "$SOURCE_DIR" && pwd)"
  else
    case "$METHOD" in
      binary)
        install_from_binary "$REPO" "$VERSION_INPUT" "$target" || die "Binary install failed. Check release assets for tag (new: anycode-${target/-unknown-/-}.tar.gz, legacy: anycode-${target}.tar.gz)."
        ;;
      source)
        install_from_git "$REPO"
        ;;
      auto)
        if install_from_binary "$REPO" "$VERSION_INPUT" "$target"; then
          :
        else
          warn "Release binary not found; falling back to cargo install --git."
          install_from_git "$REPO"
        fi
        ;;
    esac
  fi

  case ":${PATH:-}:" in
    *":${BIN_DIR}:"*) ;;
    *)
      warn "anycode installed to ${BIN_DIR}, but this directory is not on PATH."
      warn "Current shell (one-time): export PATH=\"${BIN_DIR}:\$PATH\""
      case "$(basename "${SHELL:-}")" in
        zsh) warn "Persist for zsh: echo 'export PATH=\"${BIN_DIR}:\$PATH\"' >> ~/.zshrc && source ~/.zshrc" ;;
        bash) warn "Persist for bash: echo 'export PATH=\"${BIN_DIR}:\$PATH\"' >> ~/.bashrc && source ~/.bashrc" ;;
        *) warn "Persist: add ${BIN_DIR} to your shell rc PATH." ;;
      esac
      ;;
  esac

  if [[ "$RUN_ONBOARD" -eq 1 ]]; then
    if [[ "$DRY_RUN" -eq 1 ]]; then
      info "[dry-run] ${BIN_DIR}/anycode onboard"
    else
      [[ -x "${BIN_DIR}/anycode" ]] || die "Missing ${BIN_DIR}/anycode"
      "${BIN_DIR}/anycode" onboard
    fi
  elif [[ "${ANYCODE_NO_ONBOARD:-0}" != "1" ]]; then
    info "Next: run  anycode onboard  (API 向导 + 可选微信扫码)。跳过微信:  anycode onboard --skip-wechat"
  fi
}

main
