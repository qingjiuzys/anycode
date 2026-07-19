#!/usr/bin/env bash
# Verify archive SHA-256 pin. Exits non-zero on mismatch unless bootstrap allowed.
set -euo pipefail

anycode_verify_sha256() {
  local expected="$1"
  local archive="$2"
  local actual
  actual="$(shasum -a 256 "$archive" | awk '{print $1}')"
  if [[ "$actual" == "$expected" ]]; then
    return 0
  fi
  echo "SHA-256 mismatch for $archive" >&2
  echo "  expected: $expected" >&2
  echo "  actual:   $actual" >&2
  if [[ "${ANYCODE_BENCH_ALLOW_BOOTSTRAP:-}" == "1" ]]; then
    echo "ANYCODE_BENCH_ALLOW_BOOTSTRAP=1 — continuing (update pin in download.sh)" >&2
    return 0
  fi
  return 1
}
