-- Pro: same 5h rolling window as Cloud 5h, 10000 calls/window.
-- DeepSeek V4 Pro temporarily unavailable in hosted catalog.

UPDATE cloud_plans
SET
  description = '10000 次模型调用 / 每 5 小时重置',
  quota_window_secs = 18000,
  calls_per_window = 10000
WHERE id = 'pro';

UPDATE cloud_models
SET enabled = 0
WHERE id = 'deepseek-v4-pro';

UPDATE entitlements e
INNER JOIN subscriptions s ON s.organization_id = e.organization_id
SET
  e.quota_window_secs = 18000,
  e.calls_limit_per_window = 10000,
  e.calls_used_in_window = 0,
  e.quota_window_started_at = NOW(),
  e.cloud_unlimited_rate = 0
WHERE s.plan = 'pro'
  AND s.status = 'active';
