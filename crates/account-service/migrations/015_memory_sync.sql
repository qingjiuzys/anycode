-- Opaque E2EE memory sync: ciphertext only (no server-side plaintext / dream).

CREATE TABLE IF NOT EXISTS memory_sync_envelopes (
  user_id VARCHAR(64) NOT NULL,
  envelope_id VARCHAR(128) NOT NULL,
  device_id VARCHAR(128) NOT NULL,
  ciphertext_b64 MEDIUMTEXT NOT NULL,
  nonce_b64 VARCHAR(64) NOT NULL,
  content_hash VARCHAR(128) NOT NULL,
  version_vector_json TEXT NOT NULL,
  updated_at DATETIME(3) NOT NULL,
  PRIMARY KEY (user_id, envelope_id),
  KEY idx_memory_sync_user_updated (user_id, updated_at)
);

CREATE TABLE IF NOT EXISTS memory_sync_tombstones (
  user_id VARCHAR(64) NOT NULL,
  envelope_id VARCHAR(128) NOT NULL,
  device_id VARCHAR(128) NOT NULL,
  deleted_at DATETIME(3) NOT NULL,
  PRIMARY KEY (user_id, envelope_id),
  KEY idx_memory_sync_tomb_user (user_id, deleted_at)
);
