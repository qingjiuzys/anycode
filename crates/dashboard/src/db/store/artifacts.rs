use super::*;

impl DashboardDb {
    pub async fn upsert_artifact(
        &self,
        project_id: &str,
        session_id: &str,
        path: &str,
        kind: &str,
        title: &str,
    ) -> Result<String> {
        let id = format!("art_{}_{}", project_id, path.replace('/', "_"));
        sqlx::query(
            r#"
            INSERT INTO artifacts (id, project_id, session_id, path, kind, title, trust_level, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, 'needs_verify', datetime('now'))
            ON CONFLICT(project_id, path) DO UPDATE SET
              session_id = excluded.session_id,
              title = excluded.title,
              updated_at = datetime('now')
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(session_id)
        .bind(path)
        .bind(kind)
        .bind(title)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    pub async fn list_artifacts(
        &self,
        project_id: Option<&str>,
        session_id: Option<&str>,
        kind: Option<&str>,
        trust_level: Option<&str>,
        unverified_only: bool,
        blocked_session_only: bool,
        limit: i64,
    ) -> Result<Vec<ArtifactRecord>> {
        let mut sql = String::from(
            r#"
            SELECT a.id, a.path, a.kind, a.title, a.trust_level, a.verified_by_gate_id,
                   a.session_id, a.project_id, p.name AS project_name,
                   g.name AS verified_by_gate_name, s.trusted_status AS session_trusted_status,
                   a.updated_at
            FROM artifacts a
            JOIN projects p ON p.id = a.project_id
            LEFT JOIN gates g ON g.id = a.verified_by_gate_id
            LEFT JOIN sessions s ON s.id = a.session_id
            WHERE 1=1
            "#,
        );
        if project_id.is_some() {
            sql.push_str(" AND a.project_id = ?");
        }
        if session_id.is_some() {
            sql.push_str(" AND a.session_id = ?");
        }
        if kind.is_some() {
            sql.push_str(" AND a.kind = ?");
        }
        if trust_level.filter(|t| !t.is_empty()).is_some() {
            sql.push_str(" AND a.trust_level = ?");
        }
        if unverified_only {
            sql.push_str(" AND a.trust_level NOT IN ('verified', 'trusted')");
        }
        if blocked_session_only {
            sql.push_str(" AND s.trusted_status = 'blocked'");
        }
        sql.push_str(" ORDER BY a.updated_at DESC LIMIT ?");
        let mut q = sqlx::query(&sql);
        if let Some(pid) = project_id {
            q = q.bind(pid);
        }
        if let Some(sid) = session_id {
            q = q.bind(sid);
        }
        if let Some(k) = kind {
            q = q.bind(k);
        }
        if let Some(tl) = trust_level.filter(|t| !t.is_empty()) {
            q = q.bind(tl);
        }
        let rows = q.bind(limit).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_artifact).collect())
    }
}

fn row_to_artifact(r: sqlx::sqlite::SqliteRow) -> ArtifactRecord {
    ArtifactRecord {
        id: r.get("id"),
        path: r.get("path"),
        kind: r.get("kind"),
        title: r.get("title"),
        trust_level: r.get("trust_level"),
        verified_by_gate_id: r.get("verified_by_gate_id"),
        session_id: r.get("session_id"),
        project_id: r.try_get("project_id").ok(),
        project_name: r.try_get("project_name").ok(),
        verified_by_gate_name: r.try_get("verified_by_gate_name").ok(),
        session_trusted_status: r.try_get("session_trusted_status").ok(),
        updated_at: r.try_get("updated_at").ok(),
    }
}
