-- Map hosted agnes-chat to Agnes API model id used by cpk/BYOK keys.
UPDATE cloud_models
SET upstream_model = 'agnes-2.0-flash'
WHERE id = 'agnes-chat';

-- Local free-tier dev: allow hosted models without pro plan.
UPDATE cloud_models
SET min_plan = 'free'
WHERE id = 'agnes-chat' AND min_plan = 'pro';
