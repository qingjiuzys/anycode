-- Ephemeral plaintext stream token for sender status polls (cleared on complete/fail/expire).
-- Bundle bytes are never stored. This is only the WS auth token for the active handoff TTL.

ALTER TABLE a2a_handoff_tasks
  ADD COLUMN stream_token_ephemeral VARCHAR(128) NULL AFTER stream_token_hash;
