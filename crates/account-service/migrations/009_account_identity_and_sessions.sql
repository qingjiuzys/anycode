-- Account identity, consent and rotating device sessions (MySQL 8.0+).
-- Idempotent: safe to execute repeatedly.

CREATE TABLE IF NOT EXISTS admin_users (
  id            VARCHAR(64)  NOT NULL PRIMARY KEY,
  email         VARCHAR(255) NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  role          VARCHAR(32)  NOT NULL DEFAULT 'operator',
  status        VARCHAR(32)  NOT NULL DEFAULT 'active',
  created_at    DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_admin_users_email (email)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE users ADD COLUMN email_verified_at DATETIME(3) NULL;
ALTER TABLE users ADD COLUMN identity_status VARCHAR(32) NOT NULL DEFAULT 'identity_pending';

ALTER TABLE device_links
  MODIFY COLUMN user_id VARCHAR(64) NULL;

CREATE TABLE IF NOT EXISTS email_verification_codes (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  email VARCHAR(255) NOT NULL,
  purpose VARCHAR(32) NOT NULL DEFAULT 'registration',
  code_hash VARCHAR(128) NOT NULL,
  expires_at DATETIME(3) NOT NULL,
  consumed_at DATETIME(3) NULL,
  created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  KEY idx_email_codes_lookup (email, purpose, created_at),
  KEY idx_email_codes_expiry (expires_at)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS user_consents (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  user_id VARCHAR(64) NOT NULL,
  consent_type VARCHAR(64) NOT NULL,
  policy_version VARCHAR(32) NOT NULL,
  accepted_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  withdrawn_at DATETIME(3) NULL,
  ip_hash VARCHAR(128) NULL,
  UNIQUE KEY uk_user_consent (user_id, consent_type, policy_version),
  CONSTRAINT fk_user_consents_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS identity_reviews (
  id VARCHAR(64) NOT NULL PRIMARY KEY,
  user_id VARCHAR(64) NOT NULL,
  status VARCHAR(32) NOT NULL DEFAULT 'pending',
  legal_name_ciphertext TEXT NOT NULL,
  legal_name_nonce VARCHAR(64) NOT NULL,
  id_number_ciphertext TEXT NOT NULL,
  id_number_nonce VARCHAR(64) NOT NULL,
  id_number_fingerprint VARCHAR(128) NOT NULL,
  id_number_last4 VARCHAR(8) NOT NULL,
  submitted_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  reviewed_at DATETIME(3) NULL,
  reviewer_admin_id VARCHAR(64) NULL,
  rejection_reason VARCHAR(512) NULL,
  updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_identity_review_user (user_id),
  UNIQUE KEY uk_identity_id_fingerprint (id_number_fingerprint),
  KEY idx_identity_review_status (status, submitted_at),
  CONSTRAINT fk_identity_review_user FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
  CONSTRAINT fk_identity_review_admin FOREIGN KEY (reviewer_admin_id) REFERENCES admin_users(id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE sessions ADD COLUMN session_kind VARCHAR(32) NOT NULL DEFAULT 'portal';
ALTER TABLE sessions ADD COLUMN revoked_at DATETIME(3) NULL;

ALTER TABLE linked_devices ADD COLUMN refresh_expires_at DATETIME(3) NULL;
ALTER TABLE linked_devices ADD COLUMN token_family_id VARCHAR(64) NULL;
ALTER TABLE linked_devices ADD COLUMN refresh_generation INT NOT NULL DEFAULT 0;
ALTER TABLE linked_devices ADD COLUMN previous_refresh_token_hash VARCHAR(128) NULL;
ALTER TABLE linked_devices ADD COLUMN previous_refresh_used_at DATETIME(3) NULL;
