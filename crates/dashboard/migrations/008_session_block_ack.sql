-- Session block acknowledgement: operators can mark a blocked session as
-- reviewed so it no longer counts toward the overview "blocked" banner.
ALTER TABLE sessions ADD COLUMN blocked_acknowledged_at TEXT;
