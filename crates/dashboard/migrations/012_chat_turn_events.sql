-- Canonical chat turn event log for session SSE replay and transcript hydrate.

CREATE TABLE IF NOT EXISTS chat_turn_events (
  id TEXT PRIMARY KEY NOT NULL,
  session_id TEXT NOT NULL,
  project_id TEXT NOT NULL,
  conversation_turn_id INTEGER NOT NULL,
  agent_turn INTEGER,
  seq INTEGER NOT NULL,
  kind TEXT NOT NULL,
  tool_key TEXT,
  tool_name TEXT,
  body TEXT NOT NULL DEFAULT '',
  block_json TEXT,
  payload_json TEXT NOT NULL DEFAULT '{}',
  occurred_at TEXT NOT NULL,
  UNIQUE (session_id, seq),
  FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_chat_turn_events_session_seq
  ON chat_turn_events (session_id, seq);

CREATE INDEX IF NOT EXISTS idx_chat_turn_events_session_turn
  ON chat_turn_events (session_id, conversation_turn_id, seq);
