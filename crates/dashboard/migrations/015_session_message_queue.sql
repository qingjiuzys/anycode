CREATE TABLE IF NOT EXISTS session_message_queue (
    id TEXT PRIMARY KEY NOT NULL,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    seq INTEGER NOT NULL,
    prompt TEXT NOT NULL,
    agent TEXT,
    skills_json TEXT,
    vision_json TEXT,
    text_files_json TEXT,
    lang TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    sent_at TEXT,
    error TEXT,
    UNIQUE(session_id, seq)
);

CREATE INDEX IF NOT EXISTS idx_session_message_queue_pending
    ON session_message_queue(session_id, status, seq);
