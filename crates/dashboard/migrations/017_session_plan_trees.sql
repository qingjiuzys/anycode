-- Session-scoped plan trees and todos (isolated from global orchestration.json).

CREATE TABLE IF NOT EXISTS session_plan_trees (
    session_id TEXT PRIMARY KEY NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    tree_json TEXT NOT NULL DEFAULT '{"roots":[]}',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS session_todos (
    session_id TEXT PRIMARY KEY NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    todos_json TEXT NOT NULL DEFAULT '[]',
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
