use super::*;

impl DashboardDb {
    pub async fn list_projects(&self) -> Result<Vec<ProjectSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT p.id, p.name, p.root_path, p.status, p.trust_score, p.updated_at,
                   (SELECT COUNT(*) FROM sessions s WHERE s.project_id = p.id) AS sessions_count,
                   (SELECT COUNT(*) FROM artifacts a WHERE a.project_id = p.id) AS artifacts_count
            FROM projects p
            WHERE p.organization_id = ?
            ORDER BY p.updated_at DESC
            "#,
        )
        .bind(LOCAL_ORG_ID)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ProjectSummary {
                id: r.get("id"),
                name: r.get("name"),
                root_path: r.get("root_path"),
                status: r.get("status"),
                trust_score: r.get("trust_score"),
                sessions_count: r.get("sessions_count"),
                artifacts_count: r.get("artifacts_count"),
                updated_at: r.get("updated_at"),
            })
            .collect())
    }

    pub async fn get_project(&self, project_id: &str) -> Result<Option<ProjectDetail>> {
        let row = sqlx::query(
            r#"
            SELECT id, name, root_path, description, business_goal, status, trust_score,
                   automation_level, created_at, updated_at
            FROM projects WHERE id = ?
            "#,
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| ProjectDetail {
            id: r.get("id"),
            name: r.get("name"),
            root_path: r.get("root_path"),
            description: r.get("description"),
            business_goal: r.get("business_goal"),
            status: r.get("status"),
            trust_score: r.get("trust_score"),
            automation_level: r.get("automation_level"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        }))
    }

    pub async fn upsert_project(&self, req: UpsertProjectRequest) -> Result<ProjectDetail> {
        let root = req.root_path;
        let name = req.name.unwrap_or_else(|| {
            Path::new(&root)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("project")
                .to_string()
        });
        let description = req.description.unwrap_or_default();
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT id FROM projects WHERE organization_id = ? AND root_path = ?",
        )
        .bind(LOCAL_ORG_ID)
        .bind(&root)
        .fetch_optional(&self.pool)
        .await?;
        let id = existing.unwrap_or_else(|| format!("proj_{}", Uuid::new_v4().simple()));
        sqlx::query(
            r#"
            INSERT INTO projects (id, organization_id, name, root_path, description, updated_at)
            VALUES (?, ?, ?, ?, ?, datetime('now'))
            ON CONFLICT(organization_id, root_path) DO UPDATE SET
              name = excluded.name,
              description = CASE WHEN excluded.description != '' THEN excluded.description ELSE projects.description END,
              updated_at = datetime('now')
            "#,
        )
        .bind(&id)
        .bind(LOCAL_ORG_ID)
        .bind(&name)
        .bind(&root)
        .bind(&description)
        .execute(&self.pool)
        .await?;
        self.get_project(&id)
            .await?
            .context("project missing after upsert")
    }

    pub async fn find_project_id_by_root(&self, root_path: &str) -> Result<Option<String>> {
        let row: Option<String> =
            sqlx::query_scalar("SELECT id FROM projects WHERE root_path = ? LIMIT 1")
                .bind(root_path)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row)
    }

    pub async fn list_project_ids(&self) -> Result<Vec<String>> {
        let rows: Vec<String> =
            sqlx::query_scalar("SELECT id FROM projects ORDER BY updated_at DESC")
                .fetch_all(&self.pool)
                .await?;
        Ok(rows)
    }

    pub async fn overview_stats(&self) -> Result<OverviewStats> {
        let row = sqlx::query(
            r#"
            SELECT
              (SELECT COUNT(*) FROM projects) AS projects_count,
              (SELECT COUNT(*) FROM sessions) AS sessions_total,
              (SELECT COUNT(*) FROM sessions WHERE status = 'running') AS sessions_running,
              (SELECT COUNT(*) FROM sessions WHERE trusted_status = 'blocked') AS sessions_blocked,
              (SELECT COUNT(*) FROM artifacts) AS artifacts_count,
              (SELECT COUNT(*) FROM skills) AS skills_count,
              (SELECT COUNT(*) FROM gates WHERE status = 'failed' AND required = 1) AS gates_failed,
              (SELECT COUNT(*) FROM project_events
                 WHERE occurred_at >= datetime('now', '-1 hour')) AS events_last_hour
            "#,
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(OverviewStats {
            projects_count: row.get("projects_count"),
            sessions_total: row.get("sessions_total"),
            sessions_running: row.get("sessions_running"),
            sessions_blocked: row.get("sessions_blocked"),
            artifacts_count: row.get("artifacts_count"),
            skills_count: row.get("skills_count"),
            gates_failed: row.get("gates_failed"),
            events_last_hour: row.get("events_last_hour"),
        })
    }

    pub async fn get_project_stats(&self, project_id: &str) -> Result<ProjectStats> {
        let event_types = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT event_type, COUNT(*) AS cnt FROM project_events
            WHERE project_id = ?
            GROUP BY event_type
            ORDER BY cnt DESC
            LIMIT 12
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(label, count)| LabelCount { label, count })
        .collect();

        let severities = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT severity, COUNT(*) AS cnt FROM project_events
            WHERE project_id = ?
            GROUP BY severity
            ORDER BY cnt DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(label, count)| LabelCount { label, count })
        .collect();

        let session_statuses = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT status, COUNT(*) AS cnt FROM sessions
            WHERE project_id = ?
            GROUP BY status
            ORDER BY cnt DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(label, count)| LabelCount { label, count })
        .collect();

        let gate_statuses = sqlx::query_as::<_, (String, i64)>(
            r#"
            SELECT status, COUNT(*) AS cnt FROM gates
            WHERE project_id = ?
            GROUP BY status
            ORDER BY cnt DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|(label, count)| LabelCount { label, count })
        .collect();

        let recent_failures = sqlx::query(
            r#"
            SELECT id, title, event_type, occurred_at, session_id
            FROM project_events
            WHERE project_id = ?
              AND (severity IN ('error', 'warn') OR event_type LIKE '%fail%')
            ORDER BY occurred_at DESC
            LIMIT 10
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|r| ProjectStatsFailure {
            id: r.get("id"),
            title: r.get("title"),
            event_type: r.get("event_type"),
            occurred_at: r.get("occurred_at"),
            session_id: r.get("session_id"),
        })
        .collect();

        Ok(ProjectStats {
            event_types,
            severities,
            session_statuses,
            gate_statuses,
            recent_failures,
        })
    }
}
