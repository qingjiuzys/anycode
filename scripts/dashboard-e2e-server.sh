#!/usr/bin/env bash
# Start dashboard API + embedded UI for Playwright (ephemeral DB + fixture seed).
set -euo pipefail
PORT="${1:-43199}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DB="${TMPDIR:-/tmp}/anycode-dashboard-e2e-${PORT}.db"
BIN="${ROOT}/target/release/anycode-dashboard-serve"
BASE="http://127.0.0.1:${PORT}"

echo "ensuring current anycode-dashboard-serve (embedded-ui)…" >&2
(cd "$ROOT" && ANYCODE_BUILD_DASHBOARD_UI=1 cargo build --release -p anycode-dashboard --features embedded-ui,tools-browser --bin anycode-dashboard-serve)

rm -f "$DB" "${DB}-wal" "${DB}-shm"
export ANYCODE_DASHBOARD_DB="$DB"
export ANYCODE_DASHBOARD_RECORD=0
export ANYCODE_DASHBOARD_TEST_AUTH_BYPASS=1

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

  COMPLETED_ID="$(sqlite3 "$DB" "SELECT id FROM sessions WHERE title='e2e-completed' LIMIT 1;")"
  RUNNING_ID="$(sqlite3 "$DB" "SELECT id FROM sessions WHERE title='e2e-session' LIMIT 1;")"
  sqlite3 "$DB" "UPDATE sessions SET status='completed', ended_at=datetime('now') WHERE title='e2e-completed';"
  # Seed transcript events so conversation-transcript specs have real blocks.
  NOW="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  sqlite3 "$DB" "INSERT INTO chat_turn_events
    (id, session_id, project_id, conversation_turn_id, agent_turn, seq, kind, body, block_json, payload_json, occurred_at)
  VALUES
    ('seed-u1', '${COMPLETED_ID}', '${PROJECT_ID}', 1, NULL, 1, 'user_message', 'fixture prompt',
     '{\"id\":\"seed-u1-block\",\"block_type\":\"user_message\",\"at\":\"${NOW}\",\"title\":\"You\",\"body\":\"fixture prompt\",\"meta\":{},\"collapsible\":false,\"default_collapsed\":false}',
     '{}', '${NOW}'),
    ('seed-a1', '${COMPLETED_ID}', '${PROJECT_ID}', 1, 1, 2, 'assistant_delta', 'fixture answer',
     '{\"id\":\"seed-a1-block\",\"block_type\":\"assistant_message\",\"at\":\"${NOW}\",\"title\":\"Assistant\",\"body\":\"fixture answer\",\"meta\":{},\"collapsible\":false,\"default_collapsed\":false}',
     '{}', '${NOW}'),
    ('seed-u2', '${RUNNING_ID}', '${PROJECT_ID}', 1, NULL, 1, 'user_message', 'fixture prompt',
     '{\"id\":\"seed-u2-block\",\"block_type\":\"user_message\",\"at\":\"${NOW}\",\"title\":\"You\",\"body\":\"fixture prompt\",\"meta\":{},\"collapsible\":false,\"default_collapsed\":false}',
     '{}', '${NOW}'),
    ('seed-a2', '${RUNNING_ID}', '${PROJECT_ID}', 1, 1, 2, 'assistant_delta', 'fixture answer',
     '{\"id\":\"seed-a2-block\",\"block_type\":\"assistant_message\",\"at\":\"${NOW}\",\"title\":\"Assistant\",\"body\":\"fixture answer\",\"meta\":{},\"collapsible\":false,\"default_collapsed\":false}',
     '{}', '${NOW}');"
  echo "e2e-fixture-ready"
}

"$BIN" --host 127.0.0.1 --port "$PORT" --db "$DB" &
PID=$!
cleanup() {
  trap - EXIT INT TERM
  kill "$PID" 2>/dev/null || true
  wait "$PID" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

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
