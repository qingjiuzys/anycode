-- Configurable cloud plan catalog (prices, quotas, promo labels).
CREATE TABLE IF NOT EXISTS cloud_plans (
  id VARCHAR(32) NOT NULL PRIMARY KEY,
  display_name VARCHAR(64) NOT NULL,
  description VARCHAR(255) NULL,
  monthly_price_fen INT NOT NULL DEFAULT 0,
  yearly_price_fen INT NOT NULL DEFAULT 0,
  token_limit BIGINT NOT NULL DEFAULT 0,
  api_key_limit INT NOT NULL DEFAULT 1,
  seat_limit INT NOT NULL DEFAULT 1,
  quota_window_secs INT NOT NULL DEFAULT 0,
  calls_per_window INT NOT NULL DEFAULT 0,
  hosted_models_enabled TINYINT(1) NOT NULL DEFAULT 1,
  promo_label VARCHAR(64) NULL,
  featured TINYINT(1) NOT NULL DEFAULT 0,
  enabled TINYINT(1) NOT NULL DEFAULT 1,
  sort_order INT NOT NULL DEFAULT 100,
  created_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3),
  updated_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)
);

INSERT INTO cloud_plans (
  id, display_name, description,
  monthly_price_fen, yearly_price_fen,
  token_limit, api_key_limit, seat_limit,
  quota_window_secs, calls_per_window, hosted_models_enabled,
  promo_label, featured, enabled, sort_order
) VALUES
(
  'free', 'Free', '试用 DeepSeek Flash 托管额度',
  0, 0,
  500000, 1, 1,
  0, 0, 1,
  NULL, 0, 1, 10
),
(
  'cloud_5h', 'Cloud 5h', '1000 次模型调用 / 每 5 小时重置',
  9900, 99000,
  50000000, 3, 1,
  18000, 1000, 1,
  NULL, 0, 1, 20
),
(
  'pro', 'Pro', '个人托管额度（DeepSeek ×3）',
  59900, 599000,
  15000000, 5, 1,
  0, 0, 1,
  '推荐', 1, 1, 30
),
(
  'team', 'Team', '团队席位与更高额度',
  199900, 1999000,
  60000000, 20, 10,
  0, 0, 1,
  NULL, 0, 1, 40
)
ON DUPLICATE KEY UPDATE id = id;
