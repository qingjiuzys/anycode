-- Artifact delivery flag: 1 = latest deliverable (default), 0 = intermediate scan hit.
ALTER TABLE artifacts ADD COLUMN is_final INTEGER NOT NULL DEFAULT 1;
