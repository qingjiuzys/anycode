#!/usr/bin/env bash
# Start dashboard API + embedded UI for Playwright (ephemeral DB + fixture seed).
set -euo pipefail
PORT="${1:-43199}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DB="${TMPDIR:-/tmp}/anycode-dashboard-e2e-${PORT}.db"
BIN="${ROOT}/target/release/anycode"
BASE="http://127.0.0.1:${PORT}"

if [[ ! -x "$BIN" ]]; then
  echo "release binary missing — building anycode (embedded-ui)…" >&2
  (cd "$ROOT" && ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode --features embedded-ui)
fi

rm -f "$DB" "${DB}-wal" "${DB}-shm"
export ANYCODE_DASHBOARD_DB="$DB"
export ANYCODE_DASHBOARD_RECORD=0

seed_fixture() {
  PROJECT_JSON="$(curl -sf -X POST "${BASE}/api/projects" \
    -H 'Content-Type: application/json' \
    -d "{\"root_path\":\"${ROOT}\",\"name\":\"e2e-fixture\"}")"
  PROJECT_ID="$(python3 -c "import json,sys; print(json.loads(sys.argv[1])['project']['id'])" "$PROJECT_JSON")"

  curl -sf -X POST "${BASE}/api/sessions" \
    -H 'Content-Type: application/json' \
    -d "{\"project_id\":\"${PROJECT_ID}\",\"kind\":\"run\",\"title\":\"e2e-session\"}" >/dev/null

  curl -sf -X POST "${BASE}/api/sessions" \
    -H 'Content-Type: application/json' \
    -d "{\"project_id\":\"${PROJECT_ID}\",\"kind\":\"run\",\"title\":\"e2e-completed\"}" >/dev/null

  sqlite3 "$DB" "UPDATE sessions SET status='completed', ended_at=datetime('now') WHERE title='e2e-completed';"
  echo "e2e-fixture-ready"
}

"$BIN" dashboard --host 127.0.0.1 --port "$PORT" --db "$DB" &
PID=$!

for _ in $(seq 1 90); do
  if curl -sf "${BASE}/api/health" >/dev/null 2>&1; then
    seed_fixture
    break
  fi
  sleep 1
done

if ! curl -sf "${BASE}/api/health" >/dev/null 2>&1; then
  echo "dashboard failed to become healthy on ${BASE}" >&2
  kill "$PID" 2>/dev/null || true
  exit 1
fi

wait "$PID"
