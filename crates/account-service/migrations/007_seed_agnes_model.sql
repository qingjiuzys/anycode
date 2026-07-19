-- Default hosted model for cloud gateway (free tier)
INSERT INTO cloud_models (
  id, provider_id, display_name, upstream_model,
  context_window, price_per_1m_input, price_per_1m_output, min_plan, sort_order
)
VALUES
  ('agnes-chat', 'agnes', 'Agnes Chat', 'agnes-chat', 128000, 0.00, 0.00, 'free', 0)
ON CONFLICT (id) DO UPDATE SET
  provider_id = EXCLUDED.provider_id,
  display_name = EXCLUDED.display_name,
  upstream_model = EXCLUDED.upstream_model,
  min_plan = EXCLUDED.min_plan,
  sort_order = EXCLUDED.sort_order,
  enabled = TRUE;
