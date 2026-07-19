-- Device authorization (RFC 8628 style)
CREATE TABLE IF NOT EXISTS device_links (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  device_code_hash TEXT NOT NULL UNIQUE,
  user_code TEXT NOT NULL,
  device_name TEXT,
  status TEXT NOT NULL DEFAULT 'pending',
  access_token_hash TEXT,
  refresh_token_hash TEXT,
  expires_at TIMESTAMPTZ NOT NULL,
  approved_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_device_links_user_code ON device_links(user_code);

CREATE TABLE IF NOT EXISTS linked_devices (
  id TEXT PRIMARY KEY,
  user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  device_name TEXT NOT NULL,
  platform TEXT NOT NULL DEFAULT '',
  refresh_token_hash TEXT NOT NULL UNIQUE,
  last_seen_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  revoked_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_linked_devices_user ON linked_devices(user_id);

ALTER TABLE subscriptions
  ADD COLUMN IF NOT EXISTS stripe_customer_id TEXT,
  ADD COLUMN IF NOT EXISTS stripe_subscription_id TEXT;

CREATE TABLE IF NOT EXISTS usage_events (
  id TEXT PRIMARY KEY,
  organization_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  model_id TEXT NOT NULL,
  prompt_tokens BIGINT NOT NULL DEFAULT 0,
  completion_tokens BIGINT NOT NULL DEFAULT 0,
  total_tokens BIGINT NOT NULL DEFAULT 0,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_usage_events_org ON usage_events(organization_id, created_at DESC);

CREATE TABLE IF NOT EXISTS cloud_models (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL,
  display_name TEXT NOT NULL,
  upstream_model TEXT NOT NULL,
  context_window INT NOT NULL DEFAULT 128000,
  price_per_1m_input NUMERIC(10, 4) NOT NULL DEFAULT 0,
  price_per_1m_output NUMERIC(10, 4) NOT NULL DEFAULT 0,
  min_plan TEXT NOT NULL DEFAULT 'free',
  enabled BOOLEAN NOT NULL DEFAULT TRUE,
  sort_order INT NOT NULL DEFAULT 0
);

INSERT INTO cloud_models (id, provider_id, display_name, upstream_model, context_window, price_per_1m_input, price_per_1m_output, min_plan, sort_order)
VALUES
  ('glm-4-flash', 'z.ai', 'GLM-4 Flash', 'glm-4-flash', 128000, 0.10, 0.10, 'pro', 10),
  ('gpt-4o-mini', 'openai', 'GPT-4o mini', 'gpt-4o-mini', 128000, 0.15, 0.60, 'pro', 20),
  ('claude-sonnet', 'anthropic', 'Claude Sonnet', 'claude-sonnet-4-20250514', 200000, 3.00, 15.00, 'team', 30)
ON CONFLICT (id) DO NOTHING;

ALTER TABLE entitlements
  ADD COLUMN IF NOT EXISTS hosted_models_enabled BOOLEAN NOT NULL DEFAULT FALSE,
  ADD COLUMN IF NOT EXISTS tokens_used BIGINT NOT NULL DEFAULT 0;
