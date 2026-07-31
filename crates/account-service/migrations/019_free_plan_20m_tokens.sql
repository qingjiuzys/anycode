-- Free plan: new signups get 20M hosted tokens (catalog default only).

UPDATE cloud_plans
SET
  description = '新用户赠送 2000 万 tokens（DeepSeek Flash）',
  token_limit = 20000000
WHERE id = 'free';
