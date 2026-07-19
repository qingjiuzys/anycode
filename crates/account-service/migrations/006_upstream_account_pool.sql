-- Agnes upstream account pool + admin ops tables

CREATE TABLE IF NOT EXISTS upstream_accounts (
  id                    VARCHAR(64)  NOT NULL PRIMARY KEY,
  provider_id           VARCHAR(64)  NOT NULL DEFAULT 'agnes',
  name                  VARCHAR(128) NOT NULL,
  status                VARCHAR(32)  NOT NULL DEFAULT 'active',
  weight                INT          NOT NULL DEFAULT 100,
  concurrency_limit     INT          NOT NULL DEFAULT 5,
  rpm_limit             INT          NOT NULL DEFAULT 60,
  tpm_limit             BIGINT       NOT NULL DEFAULT 1000000,
  daily_budget_tokens   BIGINT       NULL,
  monthly_budget_tokens BIGINT       NULL,
  failure_count         INT          NOT NULL DEFAULT 0,
  cooldown_until        DATETIME(3)  NULL,
  tags                  TEXT         NULL,
  notes                 TEXT         NULL,
  created_at            DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at            DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  KEY idx_upstream_accounts_provider_status (provider_id, status)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS upstream_account_keys (
  id             VARCHAR(64)  NOT NULL PRIMARY KEY,
  account_id     VARCHAR(64)  NOT NULL,
  key_ciphertext TEXT         NOT NULL,
  key_nonce      VARCHAR(64)  NOT NULL,
  base_url       VARCHAR(512) NULL,
  status         VARCHAR(32)  NOT NULL DEFAULT 'active',
  created_at     DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  revoked_at     DATETIME(3)  NULL,
  KEY idx_upstream_keys_account (account_id, status),
  CONSTRAINT fk_upstream_keys_account FOREIGN KEY (account_id) REFERENCES upstream_accounts (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS upstream_account_usage_windows (
  id                  VARCHAR(64) NOT NULL PRIMARY KEY,
  account_id          VARCHAR(64) NOT NULL,
  window_type         VARCHAR(32) NOT NULL,
  window_start        DATETIME(3) NOT NULL,
  requests_count      INT         NOT NULL DEFAULT 0,
  prompt_tokens       BIGINT      NOT NULL DEFAULT 0,
  completion_tokens   BIGINT      NOT NULL DEFAULT 0,
  KEY idx_upstream_usage_account (account_id, window_type, window_start),
  UNIQUE KEY uk_upstream_usage_window (account_id, window_type, window_start),
  CONSTRAINT fk_upstream_usage_account FOREIGN KEY (account_id) REFERENCES upstream_accounts (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS upstream_account_health_events (
  id          VARCHAR(64)  NOT NULL PRIMARY KEY,
  account_id  VARCHAR(64)  NOT NULL,
  event_type  VARCHAR(32)  NOT NULL,
  status_code INT          NULL,
  message     TEXT         NULL,
  created_at  DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  KEY idx_upstream_health_account (account_id, created_at),
  CONSTRAINT fk_upstream_health_account FOREIGN KEY (account_id) REFERENCES upstream_accounts (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS admin_users (
  id            VARCHAR(64)  NOT NULL PRIMARY KEY,
  email         VARCHAR(255) NOT NULL,
  password_hash VARCHAR(255) NOT NULL,
  role          VARCHAR(32)  NOT NULL DEFAULT 'operator',
  status        VARCHAR(32)  NOT NULL DEFAULT 'active',
  created_at    DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_admin_users_email (email)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS admin_sessions (
  id            VARCHAR(64)  NOT NULL PRIMARY KEY,
  admin_user_id VARCHAR(64)  NOT NULL,
  token_hash    VARCHAR(128) NOT NULL,
  expires_at    DATETIME(3)  NOT NULL,
  created_at    DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_admin_sessions_token (token_hash),
  KEY idx_admin_sessions_user (admin_user_id),
  CONSTRAINT fk_admin_sessions_user FOREIGN KEY (admin_user_id) REFERENCES admin_users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS admin_audit_logs (
  id            VARCHAR(64)  NOT NULL PRIMARY KEY,
  admin_user_id VARCHAR(64)  NOT NULL,
  action        VARCHAR(64)  NOT NULL,
  resource_type VARCHAR(64)  NOT NULL,
  resource_id   VARCHAR(64)  NULL,
  details       JSON         NULL,
  created_at    DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  KEY idx_admin_audit_user (admin_user_id, created_at),
  CONSTRAINT fk_admin_audit_user FOREIGN KEY (admin_user_id) REFERENCES admin_users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

ALTER TABLE usage_events
  ADD COLUMN upstream_account_id VARCHAR(64) NULL;

-- Agnes-only model catalog (replace legacy multi-provider seeds)
DELETE FROM cloud_models;
INSERT INTO cloud_models
  (id, provider_id, display_name, upstream_model, context_window, price_per_1m_input, price_per_1m_output, min_plan, sort_order)
VALUES
  ('agnes-chat', 'agnes', 'Agnes Chat', 'agnes-chat', 128000, 0.5000, 1.5000, 'pro', 10),
  ('agnes-code', 'agnes', 'Agnes Code', 'agnes-code', 128000, 0.8000, 2.0000, 'pro', 20),
  ('agnes-reasoner', 'agnes', 'Agnes Reasoner', 'agnes-reasoner', 200000, 1.2000, 3.0000, 'team', 30);
