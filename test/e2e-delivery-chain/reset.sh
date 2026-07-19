#!/usr/bin/env bash
# Phase 0: backup and wipe dashboard project/session/task data (keeps config.json).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
ANYCODE="${ANYCODE_BIN:-$ROOT/target/release/anycode}"
BACKUP_DIR="${HOME}/.anycode/backups"
STAMP="$(date +%Y%m%d-%H%M%S)"

echo "==> stopping dashboard"
pkill -f "anycode dashboard" 2>/dev/null || true
sleep 1

mkdir -p "$BACKUP_DIR"
if [[ -f "${HOME}/.anycode/projects.db" ]]; then
  echo "==> backup projects.db"
  "$ANYCODE" dashboard db backup --output "${BACKUP_DIR}/pre-e2e-${STAMP}.db" || true
fi

echo "==> remove SQLite"
rm -f "${HOME}/.anycode/projects.db" "${HOME}/.anycode/projects.db-wal" "${HOME}/.anycode/projects.db-shm"

echo "==> clear tasks + dashboard runtime"
rm -rf "${HOME}/.anycode/tasks/"*
rm -rf "${HOME}/.anycode/dashboard/"*

echo "==> reset workspace project index"
mkdir -p "${HOME}/.anycode/workspace/projects"
printf '%s\n' '{"projects":[]}' > "${HOME}/.anycode/workspace/projects/index.json"

echo "==> db check (fresh)"
"$ANYCODE" dashboard db check

echo "reset complete; config.json preserved at ${HOME}/.anycode/config.json"
