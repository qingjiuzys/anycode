-- Seed DeepSeek hosted models (token CNY = official × 3) and keep Agnes.
-- Avoid ON DUPLICATE KEY UPDATE: MySQL prepared protocol returns errno 1295 for it.

INSERT IGNORE INTO cloud_models (
  id, provider_id, display_name, upstream_model, context_window,
  price_per_1m_input, price_per_1m_output,
  price_per_1m_input_cny, price_per_1m_output_cny, currency,
  min_plan, enabled, sort_order
) VALUES
  (
    'deepseek-v4-flash', 'deepseek', 'DeepSeek V4 Flash', 'deepseek-v4-flash', 1000000,
    0.42, 0.84,
    3.0000, 6.0000, 'CNY',
    'free', 1, 5
  ),
  (
    'deepseek-v4-pro', 'deepseek', 'DeepSeek V4 Pro', 'deepseek-v4-pro', 1000000,
    1.305, 2.61,
    9.0000, 18.0000, 'CNY',
    'pro', 1, 6
  );

UPDATE cloud_models
SET
  provider_id = 'deepseek',
  display_name = 'DeepSeek V4 Flash',
  upstream_model = 'deepseek-v4-flash',
  context_window = 1000000,
  price_per_1m_input = 0.42,
  price_per_1m_output = 0.84,
  price_per_1m_input_cny = 3.0000,
  price_per_1m_output_cny = 6.0000,
  currency = 'CNY',
  min_plan = 'free',
  enabled = 1,
  sort_order = 5
WHERE id = 'deepseek-v4-flash';

UPDATE cloud_models
SET
  provider_id = 'deepseek',
  display_name = 'DeepSeek V4 Pro',
  upstream_model = 'deepseek-v4-pro',
  context_window = 1000000,
  price_per_1m_input = 1.305,
  price_per_1m_output = 2.61,
  price_per_1m_input_cny = 9.0000,
  price_per_1m_output_cny = 18.0000,
  currency = 'CNY',
  min_plan = 'pro',
  enabled = 1,
  sort_order = 6
WHERE id = 'deepseek-v4-pro';

-- Agnes remains available. Align CNY display with existing markup if null.
UPDATE cloud_models
SET
  price_per_1m_input_cny = COALESCE(price_per_1m_input_cny, 3.6000),
  price_per_1m_output_cny = COALESCE(price_per_1m_output_cny, 10.8000),
  currency = 'CNY'
WHERE id = 'agnes-chat';
