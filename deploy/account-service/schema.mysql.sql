-- anycode account-service — MySQL 8.0+ schema (InnoDB, utf8mb4)
-- Run once on empty database, e.g.:
--   mysql -h HOST -u USER -p anycode_account < deploy/account-service/schema.mysql.sql

SET NAMES utf8mb4;
SET FOREIGN_KEY_CHECKS = 0;

CREATE DATABASE IF NOT EXISTS anycode
  DEFAULT CHARACTER SET utf8mb4
  DEFAULT COLLATE utf8mb4_unicode_ci;
USE anycode;

-- ---------------------------------------------------------------------------
-- Core
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS organizations (
  id            VARCHAR(64)  NOT NULL PRIMARY KEY,
  name          VARCHAR(255) NOT NULL,
  plan_tier     VARCHAR(32)  NOT NULL DEFAULT 'free',
  sso_status    VARCHAR(32)  NOT NULL DEFAULT 'disabled',
  created_at    DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at    DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS users (
  id              VARCHAR(64)  NOT NULL PRIMARY KEY,
  organization_id VARCHAR(64)  NOT NULL,
  email           VARCHAR(255) NOT NULL,
  display_name    VARCHAR(255) NOT NULL,
  role            VARCHAR(32)  NOT NULL DEFAULT 'owner',
  password_hash   VARCHAR(255) NOT NULL,
  status          VARCHAR(32)  NOT NULL DEFAULT 'active',
  last_active_at  DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  created_at      DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at      DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_users_email (email),
  KEY idx_users_org (organization_id),
  CONSTRAINT fk_users_org FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS sessions (
  id          VARCHAR(64)  NOT NULL PRIMARY KEY,
  user_id     VARCHAR(64)  NOT NULL,
  token_hash  VARCHAR(128) NOT NULL,
  expires_at  DATETIME(3)  NOT NULL,
  created_at  DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_sessions_token (token_hash),
  KEY idx_sessions_user (user_id),
  CONSTRAINT fk_sessions_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS subscriptions (
  organization_id          VARCHAR(64)  NOT NULL PRIMARY KEY,
  plan                     VARCHAR(32)  NOT NULL DEFAULT 'free',
  status                   VARCHAR(32)  NOT NULL DEFAULT 'active',
  billing_cycle            VARCHAR(32)  NOT NULL DEFAULT 'monthly',
  period_start             DATE         NOT NULL,
  period_end               DATE         NOT NULL,
  payment_method_bound     TINYINT(1)   NOT NULL DEFAULT 0,
  stripe_customer_id       VARCHAR(128) NULL,
  stripe_subscription_id   VARCHAR(128) NULL,
  payment_provider         VARCHAR(32)  NULL,
  prepaid_until            DATE         NULL,
  pass_expires_at          DATETIME(3)  NULL,
  updated_at               DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  CONSTRAINT fk_subscriptions_org FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS entitlements (
  organization_id           VARCHAR(64) NOT NULL PRIMARY KEY,
  token_limit               BIGINT      NOT NULL,
  api_key_limit             INT         NOT NULL,
  seat_limit                INT         NOT NULL,
  hosted_models_enabled     TINYINT(1)  NOT NULL DEFAULT 0,
  tokens_used               BIGINT      NOT NULL DEFAULT 0,
  cloud_unlimited_rate      TINYINT(1)  NOT NULL DEFAULT 0,
  quota_window_secs         INT         NOT NULL DEFAULT 0,
  calls_limit_per_window    INT         NOT NULL DEFAULT 0,
  calls_used_in_window      INT         NOT NULL DEFAULT 0,
  quota_window_started_at   DATETIME(3) NULL,
  updated_at                DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  CONSTRAINT fk_entitlements_org FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS billing_contacts (
  organization_id VARCHAR(64)  NOT NULL PRIMARY KEY,
  email           VARCHAR(255) NOT NULL DEFAULT '',
  company_name    VARCHAR(255) NOT NULL DEFAULT '',
  tax_id          VARCHAR(64)  NOT NULL DEFAULT '',
  updated_at      DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3),
  CONSTRAINT fk_billing_contacts_org FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS payment_orders (
  id                 VARCHAR(64)  NOT NULL PRIMARY KEY,
  organization_id    VARCHAR(64)  NOT NULL,
  provider           VARCHAR(32)  NOT NULL,
  plan               VARCHAR(32)  NOT NULL,
  billing_cycle      VARCHAR(32)  NOT NULL DEFAULT 'monthly',
  amount_fen         INT          NOT NULL,
  amount_cents       INT          NULL COMMENT 'legacy alias; amount_fen is authoritative',
  currency           VARCHAR(8)   NOT NULL DEFAULT 'CNY',
  status             VARCHAR(32)  NOT NULL DEFAULT 'pending',
  out_trade_no       VARCHAR(64)  NOT NULL,
  provider_trade_no  VARCHAR(128) NULL,
  code_url           TEXT         NULL,
  expires_at         DATETIME(3)  NOT NULL,
  paid_at            DATETIME(3)  NULL,
  created_at         DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_payment_orders_out_trade_no (out_trade_no),
  KEY idx_payment_orders_org (organization_id, created_at),
  KEY idx_payment_orders_status (status, expires_at),
  CONSTRAINT fk_payment_orders_org FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS invoices (
  id               VARCHAR(64)   NOT NULL PRIMARY KEY,
  organization_id  VARCHAR(64)   NOT NULL,
  number           VARCHAR(64)   NOT NULL,
  period_start     DATE          NOT NULL,
  period_end       DATE          NOT NULL,
  amount_fen       INT           NOT NULL DEFAULT 0,
  currency         VARCHAR(8)    NOT NULL DEFAULT 'CNY',
  amount_usd       DECIMAL(10,2) NULL COMMENT 'legacy read-only value',
  amount_cny       DECIMAL(10,2) NULL COMMENT 'legacy decimal CNY value',
  status           VARCHAR(32)   NOT NULL DEFAULT 'draft',
  payment_order_id VARCHAR(64)   NULL,
  created_at       DATETIME(3)   NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  KEY idx_invoices_org (organization_id, created_at),
  CONSTRAINT fk_invoices_org FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE,
  CONSTRAINT fk_invoices_payment_order FOREIGN KEY (payment_order_id) REFERENCES payment_orders (id)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS cloud_api_keys (
  id              VARCHAR(64)  NOT NULL PRIMARY KEY,
  organization_id VARCHAR(64)  NOT NULL,
  name            VARCHAR(128) NOT NULL,
  prefix          VARCHAR(32)  NOT NULL,
  token_hash      VARCHAR(128) NOT NULL,
  scopes          TEXT         NOT NULL,
  expires_at      DATETIME(3)  NULL,
  revoked_at      DATETIME(3)  NULL,
  last_used_at    DATETIME(3)  NULL,
  created_at      DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  KEY idx_cloud_api_keys_org (organization_id),
  CONSTRAINT fk_cloud_api_keys_org FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

-- ---------------------------------------------------------------------------
-- Device auth + usage + model catalog
-- ---------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS device_links (
  id                   VARCHAR(64)  NOT NULL PRIMARY KEY,
  user_id              VARCHAR(64)  NOT NULL,
  device_code_hash     VARCHAR(128) NOT NULL,
  user_code            VARCHAR(16)  NOT NULL,
  device_name          VARCHAR(255) NULL,
  status               VARCHAR(32)  NOT NULL DEFAULT 'pending',
  access_token_hash    VARCHAR(128) NULL,
  refresh_token_hash   VARCHAR(128) NULL,
  expires_at           DATETIME(3)  NOT NULL,
  approved_at          DATETIME(3)  NULL,
  created_at           DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_device_links_code (device_code_hash),
  KEY idx_device_links_user_code (user_code),
  CONSTRAINT fk_device_links_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS linked_devices (
  id                 VARCHAR(64)  NOT NULL PRIMARY KEY,
  user_id            VARCHAR(64)  NOT NULL,
  device_name        VARCHAR(255) NOT NULL,
  platform           VARCHAR(64)  NOT NULL DEFAULT '',
  refresh_token_hash VARCHAR(128) NOT NULL,
  last_seen_at       DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  revoked_at         DATETIME(3)  NULL,
  created_at         DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  UNIQUE KEY uk_linked_devices_refresh (refresh_token_hash),
  KEY idx_linked_devices_user (user_id),
  CONSTRAINT fk_linked_devices_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS usage_events (
  id                VARCHAR(64) NOT NULL PRIMARY KEY,
  organization_id   VARCHAR(64) NOT NULL,
  model_id          VARCHAR(64) NOT NULL,
  prompt_tokens     BIGINT      NOT NULL DEFAULT 0,
  completion_tokens BIGINT      NOT NULL DEFAULT 0,
  total_tokens      BIGINT      NOT NULL DEFAULT 0,
  created_at        DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  KEY idx_usage_events_org (organization_id, created_at),
  CONSTRAINT fk_usage_events_org FOREIGN KEY (organization_id) REFERENCES organizations (id) ON DELETE CASCADE
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

CREATE TABLE IF NOT EXISTS cloud_models (
  id                 VARCHAR(64)    NOT NULL PRIMARY KEY,
  provider_id        VARCHAR(64)    NOT NULL,
  display_name       VARCHAR(128)   NOT NULL,
  upstream_model     VARCHAR(128)   NOT NULL,
  context_window     INT            NOT NULL DEFAULT 128000,
  price_per_1m_input DECIMAL(10,4)  NOT NULL DEFAULT 0.0000,
  price_per_1m_output DECIMAL(10,4) NOT NULL DEFAULT 0.0000,
  price_per_1m_input_cny DECIMAL(12,4) NOT NULL DEFAULT 0.0000,
  price_per_1m_output_cny DECIMAL(12,4) NOT NULL DEFAULT 0.0000,
  currency           VARCHAR(8)     NOT NULL DEFAULT 'CNY',
  min_plan           VARCHAR(32)    NOT NULL DEFAULT 'free',
  enabled            TINYINT(1)     NOT NULL DEFAULT 1,
  sort_order         INT            NOT NULL DEFAULT 0
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT IGNORE INTO cloud_models
  (id, provider_id, display_name, upstream_model, context_window,
   price_per_1m_input_cny, price_per_1m_output_cny, min_plan, sort_order)
VALUES
  ('agnes-chat', 'agnes', 'Agnes Chat', 'agnes-chat', 128000, 3.6000, 10.8000, 'pro', 10);

-- ---------------------------------------------------------------------------
-- Upstream account pool + ops admin
-- ---------------------------------------------------------------------------

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
  UNIQUE KEY uk_upstream_usage_window (account_id, window_type, window_start),
  KEY idx_upstream_usage_account (account_id, window_type, window_start),
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

SET FOREIGN_KEY_CHECKS = 1;
