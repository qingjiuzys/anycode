-- Plus (cloud_5h) and Pro token quotas + list prices (portal catalog).

UPDATE cloud_plans SET
  display_name = 'Plus',
  description = '10 亿 tokens / 月',
  monthly_price_fen = 9800,
  yearly_price_fen = 98000,
  token_limit = 1000000000,
  quota_window_secs = 0,
  calls_per_window = 0,
  promo_label = '推荐',
  featured = 1,
  sort_order = 20,
  updated_at = NOW(3)
WHERE id = 'cloud_5h';

UPDATE cloud_plans SET
  description = '100 亿 tokens / 月',
  token_limit = 10000000000,
  monthly_price_fen = 59900,
  yearly_price_fen = 599000,
  quota_window_secs = 0,
  calls_per_window = 0,
  promo_label = NULL,
  featured = 0,
  sort_order = 30,
  updated_at = NOW(3)
WHERE id = 'pro';
