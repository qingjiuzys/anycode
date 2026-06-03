-- Project knowledge base (folder index for RAG-lite search)
CREATE TABLE IF NOT EXISTS project_knowledge_paths (
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  rel_path TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  PRIMARY KEY (project_id, rel_path)
);

CREATE TABLE IF NOT EXISTS project_knowledge_chunks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  rel_path TEXT NOT NULL,
  source_file TEXT NOT NULL,
  chunk_index INTEGER NOT NULL DEFAULT 0,
  content TEXT NOT NULL,
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_knowledge_chunks_project
  ON project_knowledge_chunks(project_id);

CREATE INDEX IF NOT EXISTS idx_knowledge_chunks_source
  ON project_knowledge_chunks(project_id, source_file);
