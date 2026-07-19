-- Pending cloud compliance audit uploads (user/assistant text only).
CREATE TABLE IF NOT EXISTS compliance_audit_queue (
  id TEXT PRIMARY KEY NOT NULL,
  session_id TEXT NOT NULL,
  conversation_id TEXT NOT NULL,
  message_id TEXT NOT NULL,
  role TEXT NOT NULL,
  content TEXT NOT NULL,
  occurred_at TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_compliance_audit_queue_created
  ON compliance_audit_queue(created_at);
