-- Project-level UI prefs and extensible metadata (view_prefs, etc.)
ALTER TABLE projects ADD COLUMN metadata_json TEXT NOT NULL DEFAULT '{}';
