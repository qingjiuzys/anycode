#!/usr/bin/env bash
# One-time macOS/Linux tooling for Tauri Windows NSIS cross-compile (cargo-xwin).
# See: https://v2.tauri.app/distribute/windows-installer/
set -euo pipefail

echo "==> rustup target x86_64-pc-windows-msvc"
rustup target add x86_64-pc-windows-msvc

if ! command -v cargo-xwin >/dev/null 2>&1; then
  echo "==> install cargo-xwin"
  cargo install --locked cargo-xwin
else
  echo "==> cargo-xwin already installed: $(cargo-xwin --version 2>/dev/null || echo ok)"
fi

if [[ "$(uname -s)" == "Darwin" ]]; then
  if ! command -v makensis >/dev/null 2>&1; then
    echo "==> brew install nsis"
    brew install nsis
  else
    echo "==> nsis already installed: $(makensis -VERSION 2>/dev/null || true)"
  fi
  # Homebrew split: llvm (llvm-rc) + lld (lld-link) are separate formulae.
  if [[ ! -x /opt/homebrew/opt/llvm/bin/llvm-rc && ! -x /usr/local/opt/llvm/bin/llvm-rc ]]; then
    echo "==> brew install llvm (llvm-rc for Windows resources)"
    brew install llvm || true
  else
    echo "==> llvm already installed"
  fi
  if ! command -v lld-link >/dev/null 2>&1; then
    echo "==> brew install lld (lld-link for MSVC target)"
    brew install lld
  else
    echo "==> lld already installed: $(lld-link --version 2>/dev/null | head -1 || true)"
  fi
  echo
  echo "Add LLVM to PATH for builds (Apple Silicon Homebrew):"
  echo '  export PATH="/opt/homebrew/opt/llvm/bin:$PATH"'
elif [[ "$(uname -s)" == "Linux" ]]; then
  echo "On Linux, ensure: nsis, lld, llvm/clang (distro packages)."
  echo "Example (Debian/Ubuntu): sudo apt install nsis lld llvm clang"
fi

echo
echo "Ready. Build with:"
echo "  ./scripts/release-desktop-windows-cross.sh"
