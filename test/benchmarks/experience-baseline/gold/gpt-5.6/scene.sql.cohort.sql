SELECT
    u.org_id,
    COUNT(DISTINCT u.id) AS activated_users
FROM users AS u
JOIN events AS e
    ON e.user_id = u.id
   AND e.event_name = 'activate'
   AND e.created_at >= u.created_at
   AND e.created_at <= u.created_at + INTERVAL '7 days'
GROUP BY u.org_id
ORDER BY activated_users DESC
LIMIT 20;
