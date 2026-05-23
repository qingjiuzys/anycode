use super::*;

impl DashboardDb {
    pub async fn upsert_skill(
        &self,
        id: &str,
        name: &str,
        description: &str,
        source_path: &str,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO skills (id, name, description, source_path)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              name = excluded.name,
              description = excluded.description,
              source_path = excluded.source_path,
              updated_at = datetime('now')
            "#,
        )
        .bind(id)
        .bind(name)
        .bind(description)
        .bind(source_path)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn link_project_skill(
        &self,
        project_id: &str,
        skill_id: &str,
        enabled: bool,
    ) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO project_skills (project_id, skill_id, enabled)
            VALUES (?, ?, ?)
            ON CONFLICT(project_id, skill_id) DO UPDATE SET enabled = excluded.enabled
            "#,
        )
        .bind(project_id)
        .bind(skill_id)
        .bind(i64::from(enabled))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn project_skill_enabled_count(&self) -> Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project_skills WHERE enabled = 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(n)
    }

    pub async fn list_skills(&self, limit: i64) -> Result<Vec<SkillRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT s.id, s.name, s.description, s.source_path,
                   (SELECT COUNT(*) FROM project_skills ps WHERE ps.skill_id = s.id AND ps.enabled = 1) AS projects_count
            FROM skills s
            ORDER BY s.name
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| SkillRecord {
                id: r.get("id"),
                name: r.get("name"),
                description: r.get("description"),
                source_path: r.get("source_path"),
                projects_count: r.get("projects_count"),
                enabled: None,
            })
            .collect())
    }

    pub async fn list_skills_for_project(&self, project_id: &str) -> Result<Vec<SkillRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT s.id, s.name, s.description, s.source_path,
                   (SELECT COUNT(*) FROM project_skills ps2 WHERE ps2.skill_id = s.id AND ps2.enabled = 1) AS projects_count,
                   COALESCE(ps.enabled, 0) AS project_enabled
            FROM skills s
            LEFT JOIN project_skills ps ON ps.skill_id = s.id AND ps.project_id = ?
            ORDER BY s.name
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| SkillRecord {
                id: r.get("id"),
                name: r.get("name"),
                description: r.get("description"),
                source_path: r.get("source_path"),
                projects_count: r.get("projects_count"),
                enabled: Some(r.get::<i64, _>("project_enabled") != 0),
            })
            .collect())
    }
}
