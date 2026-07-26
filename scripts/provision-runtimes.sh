#!/usr/bin/env bash
# Provision anyCode's managed runtimes under ~/.anycode/runtimes:
#   python/bin/python3  — managed CPython (via uv, python-build-standalone)
#   node/bin/node       — official Node.js distribution
# Idempotent: exits fast when runtimes already exist. Falls back to system
# runtimes (reports only) when network or platform support is unavailable.
set -euo pipefail

RT="${ANYCODE_RUNTIMES_DIR:-$HOME/.anycode/runtimes}"
NODE_VERSION="${ANYCODE_NODE_VERSION:-v22.18.0}"
PYTHON_VERSION="${ANYCODE_PYTHON_VERSION:-3.12}"
mkdir -p "$RT/bin"

log() { printf '[provision-runtimes] %s\n' "$*"; }

detect_os_arch() {
  OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
  ARCH="$(uname -m)"
  case "$ARCH" in
    x86_64|amd64) ARCH=x64 ;;
    aarch64|arm64) ARCH=arm64 ;;
    *) log "unsupported arch: $ARCH"; ARCH="" ;;
  esac
}

# ---------------------------------------------------------------- Python ---
provision_python() {
  if [ -x "$RT/python/bin/python3" ]; then
    log "python already provisioned: $RT/python/bin/python3"
    return 0
  fi
  if command -v uv >/dev/null 2>&1; then
    UV_BIN="$(command -v uv)"
  elif [ -x "$RT/bin/uv" ]; then
    UV_BIN="$RT/bin/uv"
  else
    log "installing uv (standalone)..."
    if command -v curl >/dev/null 2>&1; then
      curl -LsSf https://astral.sh/uv/install.sh | env UV_INSTALL_DIR="$RT/bin" UV_UNMANAGED_INSTALL=1 sh || UV_BIN=""
    fi
    UV_BIN="${UV_BIN:-$RT/bin/uv}"
  fi
  if [ -n "${UV_BIN:-}" ] && [ -x "$UV_BIN" ]; then
    log "installing managed Python $PYTHON_VERSION via uv..."
    if "$UV_BIN" python install "$PYTHON_VERSION" --install-dir "$RT/python"; then
      PY_BIN="$("$UV_BIN" python find "$PYTHON_VERSION" --install-dir "$RT/python" 2>/dev/null || true)"
      if [ -n "$PY_BIN" ] && [ -x "$PY_BIN" ]; then
        mkdir -p "$RT/python/bin"
        ln -sf "$PY_BIN" "$RT/python/bin/python3"
        ln -sf "$PY_BIN" "$RT/python/bin/python"
        log "python ready: $("$RT/python/bin/python3" --version 2>&1)"
        return 0
      fi
    fi
    log "uv python install failed; falling back to system python3"
  else
    log "uv unavailable (offline?); falling back to system python3"
  fi
  if command -v python3 >/dev/null 2>&1; then
    mkdir -p "$RT/python/bin"
    ln -sf "$(command -v python3)" "$RT/python/bin/python3"
    log "linked system python3: $(python3 --version 2>&1)"
  else
    log "WARNING: no python3 available on this machine"
  fi
}

# ------------------------------------------------------------------ Node ---
provision_node() {
  if [ -x "$RT/node/bin/node" ]; then
    log "node already provisioned: $RT/node/bin/node"
    return 0
  fi
  detect_os_arch
  if [ "$OS" != "darwin" ] && [ "$OS" != "linux" ]; then
    log "node tarball provisioning supports macOS/Linux only (os=$OS)"
  elif [ -n "$ARCH" ] && command -v curl >/dev/null 2>&1; then
    TARBALL="node-${NODE_VERSION}-${OS}-${ARCH}.tar.gz"
    URL="https://nodejs.org/dist/${NODE_VERSION}/${TARBALL}"
    SUMS_URL="https://nodejs.org/dist/${NODE_VERSION}/SHASUMS256.txt"
    TMP_DIR="$(mktemp -d)"
    trap 'rm -rf "$TMP_DIR"' EXIT
    log "downloading $URL ..."
    if curl -fsSL "$URL" -o "$TMP_DIR/$TARBALL" && curl -fsSL "$SUMS_URL" -o "$TMP_DIR/SHASUMS256.txt"; then
      EXPECTED="$(grep " ${TARBALL}\$" "$TMP_DIR/SHASUMS256.txt" | awk '{print $1}')"
      ACTUAL="$(shasum -a 256 "$TMP_DIR/$TARBALL" | awk '{print $1}')"
      if [ -n "$EXPECTED" ] && [ "$EXPECTED" != "$ACTUAL" ]; then
        log "ERROR: node tarball checksum mismatch"; return 1
      fi
      mkdir -p "$RT/node"
      tar -xzf "$TMP_DIR/$TARBALL" -C "$RT/node" --strip-components=1
      log "node ready: $("$RT/node/bin/node" --version 2>&1)"
      return 0
    fi
    log "node download failed (offline?)"
  fi
  if command -v node >/dev/null 2>&1; then
    mkdir -p "$RT/node/bin"
    ln -sf "$(command -v node)" "$RT/node/bin/node"
    log "linked system node: $(node --version 2>&1)"
  else
    log "WARNING: no node available on this machine"
  fi
}

log "runtimes root: $RT"
provision_python
provision_node
log "done"
