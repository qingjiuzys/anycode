-- Encrypted cloud-conversation compliance audit (MySQL 8.0+).
-- Offline/local-only conversations are intentionally outside this schema.

CREATE TABLE IF NOT EXISTS audit_keyword_rules (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  name VARCHAR(128) NOT NULL,
  keyword VARCHAR(255) NOT NULL,
  severity VARCHAR(32) NOT NULL DEFAULT 'review',
  enabled TINYINT(1) NOT NULL DEFAULT 1,
  created_by_admin_id VARCHAR(64) NULL,
  created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  KEY idx_audit_rules_enabled (enabled),
  CONSTRAINT fk_audit_rule_admin FOREIGN KEY (created_by_admin_id) REFERENCES admin_users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS audit_conversations (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  organization_id VARCHAR(64) NOT NULL,
  user_id VARCHAR(64) NOT NULL,
  client_conversation_id VARCHAR(128) NOT NULL,
  source VARCHAR(32) NOT NULL DEFAULT 'cloud',
  started_at DATETIME(3) NOT NULL,
  last_message_at DATETIME(3) NOT NULL,
  expires_at DATETIME(3) NOT NULL,
  created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_audit_conversation_client (organization_id, client_conversation_id),
  KEY idx_audit_conversation_expiry (expires_at),
  CONSTRAINT fk_audit_conversation_org FOREIGN KEY (organization_id) REFERENCES organizations(id) ON DELETE CASCADE,
  CONSTRAINT fk_audit_conversation_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS audit_messages (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  conversation_id VARCHAR(64) NOT NULL,
  client_message_id VARCHAR(128) NOT NULL,
  role VARCHAR(32) NOT NULL,
  content_ciphertext MEDIUMTEXT NOT NULL,
  content_nonce VARCHAR(64) NOT NULL,
  encrypted_data_key TEXT NOT NULL,
  data_key_nonce VARCHAR(64) NOT NULL,
  content_sha256 VARCHAR(128) NOT NULL,
  occurred_at DATETIME(3) NOT NULL,
  expires_at DATETIME(3) NOT NULL,
  created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_audit_message_client (conversation_id, client_message_id),
  KEY idx_audit_message_expiry (expires_at),
  CONSTRAINT fk_audit_message_conversation FOREIGN KEY (conversation_id) REFERENCES audit_conversations(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS audit_keyword_hits (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  message_id VARCHAR(64) NOT NULL,
  rule_id VARCHAR(64) NOT NULL,
  severity VARCHAR(32) NOT NULL,
  matched_excerpt_masked VARCHAR(512) NOT NULL,
  created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_audit_hit (message_id, rule_id),
  KEY idx_audit_hit_created (created_at),
  CONSTRAINT fk_audit_hit_message FOREIGN KEY (message_id) REFERENCES audit_messages(id) ON DELETE CASCADE,
  CONSTRAINT fk_audit_hit_rule FOREIGN KEY (rule_id) REFERENCES audit_keyword_rules(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS audit_access_logs (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  admin_user_id VARCHAR(64) NOT NULL,
  action VARCHAR(64) NOT NULL,
  resource_type VARCHAR(64) NOT NULL,
  resource_id VARCHAR(64) NULL,
  purpose VARCHAR(255) NOT NULL,
  created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  KEY idx_audit_access_admin (admin_user_id, created_at),
  CONSTRAINT fk_audit_access_admin FOREIGN KEY (admin_user_id) REFERENCES admin_users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;
