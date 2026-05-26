use super::*;
use crate::project_root::{normalize_project_root, project_id_for_root};
use std::collections::HashMap;

fn row_to_project_summary(r: sqlx::sqlite::SqliteRow) -> ProjectSummary {
    let root_path: String = r.get("root_path");
    ProjectSummary {
        id: r.get("id"),
        name: r.get("name"),
        root_path: root_path.clone(),
        status: r.get("status"),
        trust_score: r.get("trust_score"),
        sessions_count: r.get("sessions_count"),
        artifacts_count: r.get("artifacts_count"),
        updated_at: r.get("updated_at"),
        root_exists: std::path::Path::new(&root_path).is_dir(),
    }
}

impl DashboardDb {
    pub async fn list_projects(&self) -> Result<Vec<ProjectSummary>> {
        let (projects, _) = self
            .list_projects_paged(None, None, 10_000, 0, "updated_at_desc")
            .await?;
        Ok(projects)
    }

    pub async fn list_projects_paged(
        &self,
        q: Option<&str>,
        status: Option<&str>,
        limit: i64,
        offset: i64,
        sort: &str,
    ) -> Result<(Vec<ProjectSummary>, i64)> {
        let _ = self.repair_project_identity().await?;
        let limit = limit.clamp(1, 500);
        let offset = offset.max(0);
        let mut conditions = vec!["p.organization_id = ?".to_string()];
        if q.filter(|s| !s.trim().is_empty()).is_some() {
            conditions.push("(p.name LIKE ? OR p.root_path LIKE ?)".into());
        }
        if status.filter(|s| !s.is_empty()).is_some() {
            conditions.push("p.status = ?".into());
        }
        let where_clause = conditions.join(" AND ");
        let order_by = match sort {
            "name_asc" => "p.name ASC",
            "name_desc" => "p.name DESC",
            "sessions_desc" => "sessions_count DESC, p.updated_at DESC",
            "updated_at_asc" => "p.updated_at ASC",
            _ => "p.updated_at DESC",
        };
        let count_sql = format!("SELECT COUNT(*) AS cnt FROM projects p WHERE {where_clause}");
        let list_sql = format!(
            r#"
            SELECT p.id, p.name, p.root_path, p.status, p.trust_score, p.updated_at,
                   (SELECT COUNT(*) FROM sessions s WHERE s.project_id = p.id) AS sessions_count,
                   (SELECT COUNT(*) FROM artifacts a WHERE a.project_id = p.id) AS artifacts_count
            FROM projects p
            WHERE {where_clause}
            ORDER BY {order_by}
            LIMIT ? OFFSET ?
            "#
        );

        let like = q
            .filter(|s| !s.trim().is_empty())
            .map(|s| format!("%{}%", s.trim()));

        let mut count_q = sqlx::query(&count_sql).bind(LOCAL_ORG_ID);
        if let Some(ref pattern) = like {
            count_q = count_q.bind(pattern).bind(pattern);
        }
        if let Some(st) = status.filter(|s| !s.is_empty()) {
            count_q = count_q.bind(st);
        }
        let total: i64 = count_q.fetch_one(&self.pool).await?.get("cnt");

        let mut list_q = sqlx::query(&list_sql).bind(LOCAL_ORG_ID);
        if let Some(ref pattern) = like {
            list_q = list_q.bind(pattern).bind(pattern);
        }
        if let Some(st) = status.filter(|s| !s.is_empty()) {
            list_q = list_q.bind(st);
        }
        let rows = list_q
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        Ok((
            rows.into_iter().map(row_to_project_summary).collect(),
            total,
        ))
    }

    pub async fn set_project_status(&self, project_id: &str, status: &str) -> Result<bool> {
        let result = sqlx::query(
            "UPDATE projects SET status = ?, updated_at = datetime('now') WHERE id = ?",
        )
        .bind(status)
        .bind(project_id)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
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
        let root = normalize_project_root(Path::new(&req.root_path))?
            .display()
            .to_string();
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
        let id = existing.unwrap_or_else(|| project_id_for_root(&root));
        sqlx::query(
            r#"
            INSERT INTO projects (id, organization_id, name, root_path, description, updated_at)
            VALUES (?, ?, ?, ?, ?, datetime('now'))
            ON CONFLICT(organization_id, root_path) DO UPDATE SET
              name = excluded.name,
              description = CASE WHEN excluded.description != '' THEN excluded.description ELSE projects.description END,
              updated_at = datetime('now')
            ON CONFLICT(id) DO UPDATE SET
              root_path = excluded.root_path,
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

    pub async fn repair_project_identity(&self) -> Result<u64> {
        #[derive(Debug, Clone)]
        struct ProjectRow {
            id: String,
            name: String,
            root_path: String,
            description: String,
            business_goal: String,
            automation_level: i64,
            status: String,
            trust_score: f64,
            created_at: String,
            updated_at: String,
        }

        let rows = sqlx::query(
            r#"
            SELECT id, name, root_path, description, business_goal, automation_level, status,
                   trust_score, created_at, updated_at
            FROM projects
            WHERE organization_id = ?
            "#,
        )
        .bind(LOCAL_ORG_ID)
        .fetch_all(&self.pool)
        .await?;

        let mut by_root: HashMap<String, Vec<ProjectRow>> = HashMap::new();
        for r in rows {
            let raw_root: String = r.get("root_path");
            let normalized = normalize_project_root(Path::new(&raw_root))
                .map(|p| p.display().to_string())
                .unwrap_or(raw_root);
            by_root.entry(normalized).or_default().push(ProjectRow {
                id: r.get("id"),
                name: r.get("name"),
                root_path: r.get("root_path"),
                description: r.get("description"),
                business_goal: r.get("business_goal"),
                automation_level: r.get("automation_level"),
                status: r.get("status"),
                trust_score: r.get("trust_score"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            });
        }

        let mut repaired = 0u64;
        for (normalized_root, mut group) in by_root {
            group.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            let winner = group[0].clone();
            let stable_id = project_id_for_root(&normalized_root);
            let canonical_id = group
                .iter()
                .find(|row| row.id == stable_id)
                .or_else(|| group.iter().find(|row| row.root_path == normalized_root))
                .map(|row| row.id.clone())
                .unwrap_or(stable_id);
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO projects
                  (id, organization_id, name, root_path, description, business_goal,
                   automation_level, status, trust_score, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&canonical_id)
            .bind(LOCAL_ORG_ID)
            .bind(&winner.name)
            .bind(&normalized_root)
            .bind(&winner.description)
            .bind(&winner.business_goal)
            .bind(winner.automation_level)
            .bind(&winner.status)
            .bind(winner.trust_score)
            .bind(&winner.created_at)
            .bind(&winner.updated_at)
            .execute(&self.pool)
            .await?;

            sqlx::query(
                r#"
                UPDATE projects
                SET root_path = ?, name = ?, description = ?, business_goal = ?,
                    automation_level = ?, status = ?, trust_score = ?, updated_at = ?
                WHERE id = ?
                "#,
            )
            .bind(&normalized_root)
            .bind(&winner.name)
            .bind(&winner.description)
            .bind(&winner.business_goal)
            .bind(winner.automation_level)
            .bind(&winner.status)
            .bind(winner.trust_score)
            .bind(&winner.updated_at)
            .bind(&canonical_id)
            .execute(&self.pool)
            .await?;

            for row in group {
                if row.id == canonical_id {
                    if row.root_path != normalized_root {
                        repaired += 1;
                    }
                    continue;
                }
                self.move_project_references(&row.id, &canonical_id).await?;
                sqlx::query("DELETE FROM projects WHERE id = ?")
                    .bind(&row.id)
                    .execute(&self.pool)
                    .await?;
                repaired += 1;
            }
        }
        Ok(repaired)
    }

    async fn move_project_references(&self, old_id: &str, new_id: &str) -> Result<()> {
        for table in [
            "sessions",
            "agents",
            "project_events",
            "gates",
            "automation_policies",
            "asset_sources",
            "asset_permissions",
            "asset_index_jobs",
            "notification_policies",
            "skill_runs",
        ] {
            let sql = format!("UPDATE {table} SET project_id = ? WHERE project_id = ?");
            sqlx::query(&sql)
                .bind(new_id)
                .bind(old_id)
                .execute(&self.pool)
                .await?;
        }

        sqlx::query(
            r#"
            INSERT OR IGNORE INTO project_skills (project_id, skill_id, enabled, config_json)
            SELECT ?, skill_id, enabled, config_json FROM project_skills WHERE project_id = ?
            "#,
        )
        .bind(new_id)
        .bind(old_id)
        .execute(&self.pool)
        .await?;
        sqlx::query("DELETE FROM project_skills WHERE project_id = ?")
            .bind(old_id)
            .execute(&self.pool)
            .await?;

        sqlx::query(
            r#"
            DELETE FROM artifacts
            WHERE project_id = ?
              AND path IN (SELECT path FROM artifacts WHERE project_id = ?)
            "#,
        )
        .bind(old_id)
        .bind(new_id)
        .execute(&self.pool)
        .await?;
        sqlx::query("UPDATE artifacts SET project_id = ? WHERE project_id = ?")
            .bind(new_id)
            .bind(old_id)
            .execute(&self.pool)
            .await?;

        sqlx::query(
            r#"
            DELETE FROM metrics_daily
            WHERE project_id = ?
              AND date IN (SELECT date FROM metrics_daily WHERE project_id = ?)
            "#,
        )
        .bind(old_id)
        .bind(new_id)
        .execute(&self.pool)
        .await?;
        sqlx::query("UPDATE metrics_daily SET project_id = ? WHERE project_id = ?")
            .bind(new_id)
            .bind(old_id)
            .execute(&self.pool)
            .await?;

        Ok(())
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
              (SELECT COUNT(DISTINCT session_id) FROM project_events
                 WHERE event_type = 'budget_exceeded'
                   AND session_id IS NOT NULL
                   AND occurred_at >= datetime('now', '-7 days')) AS sessions_budget_exceeded,
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
            sessions_budget_exceeded: row.get("sessions_budget_exceeded"),
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
