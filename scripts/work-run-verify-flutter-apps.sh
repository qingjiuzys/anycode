#!/usr/bin/env bash
# Legacy Flutter work-run gates removed. Use the canonical E2E suite:
#   node test/target/run_all.mjs
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "Flutter work-run apps (test-flutter-*, test/app-*) were removed."
echo "Run the target E2E suite instead:"
echo "  node test/target/run_all.mjs"
exec node test/target/run_all.mjs
