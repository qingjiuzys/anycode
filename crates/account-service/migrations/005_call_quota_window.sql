-- Model call quota per rolling window (e.g. 1000 calls / 5 hours)
ALTER TABLE entitlements
  ADD COLUMN IF NOT EXISTS quota_window_secs INT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS calls_limit_per_window INT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS calls_used_in_window INT NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS quota_window_started_at TIMESTAMPTZ;
