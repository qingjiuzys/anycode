-- Session SSOT: dedupe task_id, unique index, backfill imported titles from index events.

-- Remove duplicate sessions for the same task_id (keep newest started_at).
DELETE FROM sessions
WHERE id IN (
  SELECT s.id
  FROM sessions s
  JOIN (
    SELECT task_id, MAX(started_at) AS keep_started
    FROM sessions
    WHERE task_id IS NOT NULL AND TRIM(task_id) != ''
    GROUP BY task_id
    HAVING COUNT(*) > 1
  ) d ON s.task_id = d.task_id AND s.started_at < d.keep_started
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_sessions_task_id
  ON sessions(task_id)
  WHERE task_id IS NOT NULL AND TRIM(task_id) != '';

-- Backfill placeholder titles from earliest user_prompt index event (SQLite only).
UPDATE sessions
SET
  title = (
    SELECT SUBSTR(e.body, 1, 120)
    FROM project_events e
    WHERE e.session_id = sessions.id
      AND e.event_type = 'user_prompt'
      AND TRIM(e.body) != ''
    ORDER BY e.occurred_at ASC
    LIMIT 1
  ),
  prompt_preview = (
    SELECT SUBSTR(e.body, 1, 240)
    FROM project_events e
    WHERE e.session_id = sessions.id
      AND e.event_type = 'user_prompt'
      AND TRIM(e.body) != ''
    ORDER BY e.occurred_at ASC
    LIMIT 1
  )
WHERE title LIKE 'Imported task %'
  AND EXISTS (
    SELECT 1 FROM project_events e
    WHERE e.session_id = sessions.id
      AND e.event_type = 'user_prompt'
      AND TRIM(e.body) != ''
  );
