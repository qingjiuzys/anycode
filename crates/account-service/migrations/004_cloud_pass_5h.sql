-- Time-based cloud pass (e.g. 5-hour hosted inference package)
ALTER TABLE subscriptions
  ADD COLUMN IF NOT EXISTS pass_expires_at TIMESTAMPTZ;

ALTER TABLE entitlements
  ADD COLUMN IF NOT EXISTS cloud_unlimited_rate BOOLEAN NOT NULL DEFAULT FALSE;
