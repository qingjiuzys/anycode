#!/usr/bin/env bash
# Collect Rust (llvm-cov) and Dashboard UI (Vitest) coverage snapshots.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/test/results/.coverage}"
mkdir -p "$OUT"
OUT="$(cd "$OUT" && pwd)"

summary() {
  python3 - "$OUT/summary.json" <<'PY'
import json, sys
from pathlib import Path
summary_path = Path(sys.argv[1])
out = summary_path.parent
payload = {"rust": None, "dashboard_ui": None}
rust_lcov = out / "rust.lcov"
if rust_lcov.exists():
    payload["rust"] = {"lcov": str(rust_lcov), "bytes": rust_lcov.stat().st_size}
ui_summary = out / "dashboard-ui" / "coverage-summary.json"
if ui_summary.exists():
    payload["dashboard_ui"] = json.loads(ui_summary.read_text())
summary_path.write_text(json.dumps(payload, indent=2) + "\n")
PY
}

if [[ "${ANYCODE_COVERAGE_FORCE:-0}" != "1" ]] \
  && [[ -s "$OUT/rust.lcov" ]] \
  && [[ -s "$OUT/account-service.lcov" ]] \
  && [[ -s "$OUT/dashboard-ui/coverage-summary.json" ]]; then
  summary
  echo "coverage cache reused from $OUT (set ANYCODE_COVERAGE_FORCE=1 to rebuild)" >&2
  exit 0
fi

if ! cargo llvm-cov --version >/dev/null 2>&1; then
  echo "installing cargo-llvm-cov…" >&2
  cargo install cargo-llvm-cov --locked
fi

echo "==> Rust workspace llvm-cov" >&2
(
  cd "$ROOT"
  cargo llvm-cov --workspace --lcov --output-path "$OUT/rust.lcov" --fail-under-lines 0
)

echo "==> account-service llvm-cov" >&2
(
  cd "$ROOT/crates/account-service"
  cargo llvm-cov --all-targets --lcov --output-path "$OUT/account-service.lcov" --fail-under-lines 0
)

echo "==> Dashboard UI vitest coverage" >&2
(
  cd "$ROOT/crates/dashboard-ui"
  if ! npm ls @vitest/coverage-v8 >/dev/null 2>&1; then
    npm install --no-save @vitest/coverage-v8@^3.0.5
  fi
  npx vitest run --coverage --coverage.reporter=json-summary --coverage.reporter=lcov \
    --coverage.reportsDirectory="$OUT/dashboard-ui"
)

summary
echo "coverage written to $OUT" >&2
