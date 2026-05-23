#!/usr/bin/env bash
# Install anycode from local clone with Digital Workbench UI embedded.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
exec bash "${ROOT}/scripts/install.sh" --method source --source-dir "$ROOT" --with-dashboard "$@"
