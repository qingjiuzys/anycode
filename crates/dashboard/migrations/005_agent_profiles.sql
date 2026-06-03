-- Agent profile definitions (declarative, synced from config.json)

CREATE TABLE IF NOT EXISTS agent_profiles (
  id TEXT PRIMARY KEY,
  scope TEXT NOT NULL DEFAULT 'global',
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  extends TEXT NOT NULL DEFAULT 'general-purpose',
  description TEXT NOT NULL DEFAULT '',
  tools_json TEXT NOT NULL DEFAULT '{}',
  skills_json TEXT NOT NULL DEFAULT '{}',
  routing_json TEXT NOT NULL DEFAULT '{}',
  prompt_overlay TEXT NOT NULL DEFAULT '',
  version INTEGER NOT NULL DEFAULT 1,
  builtin INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_agent_profiles_scope
  ON agent_profiles(scope, project_id);

CREATE TABLE IF NOT EXISTS project_agent_bindings (
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  profile_id TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1,
  is_default_run INTEGER NOT NULL DEFAULT 0,
  is_default_goal INTEGER NOT NULL DEFAULT 0,
  config_json TEXT NOT NULL DEFAULT '{}',
  PRIMARY KEY (project_id, profile_id)
);
