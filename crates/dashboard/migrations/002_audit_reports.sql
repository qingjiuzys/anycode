-- Dashboard audit query indexes (auth_events reused as workbench audit log)
CREATE INDEX IF NOT EXISTS idx_auth_events_source_time
  ON auth_events(source, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_auth_events_event_type
  ON auth_events(event_type, created_at DESC);
