-- 008: Cloud catalog trim — only Cloud Auto + Agnes Chat
-- Idempotent migration with audit record.

CREATE TABLE IF NOT EXISTS schema_migrations (
  id          VARCHAR(64)  NOT NULL PRIMARY KEY,
  applied_at  DATETIME(3)  NOT NULL DEFAULT CURRENT_TIMESTAMP(3)
) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4 COLLATE=utf8mb4_unicode_ci;

INSERT IGNORE INTO schema_migrations (id) VALUES ('008_cloud_catalog_trim');

-- Disable retired models (keep rows for usage history)
UPDATE cloud_models SET enabled = 0 WHERE id IN ('agnes-code', 'agnes-reasoner');

-- Ensure Agnes Chat is the sole hosted named model
INSERT INTO cloud_models
  (id, provider_id, display_name, upstream_model, context_window, price_per_1m_input, price_per_1m_output, min_plan, enabled, sort_order)
VALUES
  ('agnes-chat', 'agnes', 'Agnes Chat', 'agnes-chat', 128000, 0.5000, 1.5000, 'pro', 1, 10)
ON DUPLICATE KEY UPDATE
  enabled = 1,
  display_name = VALUES(display_name),
  upstream_model = VALUES(upstream_model),
  sort_order = VALUES(sort_order);

-- Rollback (manual):
-- UPDATE cloud_models SET enabled = 1 WHERE id IN ('agnes-code', 'agnes-reasoner');
