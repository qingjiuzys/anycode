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
        self.upsert_artifact_with_final(project_id, session_id, path, kind, title, true)
            .await
    }

    pub async fn upsert_artifact_scanned(
        &self,
        project_id: &str,
        session_id: &str,
        path: &str,
        kind: &str,
        title: &str,
    ) -> Result<String> {
        self.upsert_artifact_with_final(project_id, session_id, path, kind, title, false)
            .await
    }

    async fn upsert_artifact_with_final(
        &self,
        project_id: &str,
        session_id: &str,
        path: &str,
        kind: &str,
        title: &str,
        is_final: bool,
    ) -> Result<String> {
        let id = format!("art_{}_{}", project_id, path.replace('/', "_"));
        let session_id = if session_id.is_empty() {
            None
        } else {
            Some(session_id)
        };
        let existing_meta: Option<String> = sqlx::query_scalar(
            "SELECT metadata_json FROM artifacts WHERE project_id = ? AND path = ?",
        )
        .bind(project_id)
        .bind(path)
        .fetch_optional(&self.pool)
        .await?;
        let version = existing_meta
            .as_deref()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
            .and_then(|v| v.get("version").and_then(|n| n.as_u64()))
            .map(|n| n + 1)
            .unwrap_or(1);
        let metadata_json = serde_json::json!({
            "version": version,
            "is_final": is_final,
            "updated_session_id": session_id,
        })
        .to_string();
        let is_final_i: i64 = if is_final { 1 } else { 0 };
        sqlx::query(
            r#"
            INSERT INTO artifacts (id, project_id, session_id, path, kind, title, trust_level, metadata_json, is_final, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, 'needs_verify', ?, ?, datetime('now'))
            ON CONFLICT(project_id, path) DO UPDATE SET
              session_id = excluded.session_id,
              title = excluded.title,
              metadata_json = excluded.metadata_json,
              is_final = excluded.is_final,
              updated_at = datetime('now')
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(session_id)
        .bind(path)
        .bind(kind)
        .bind(title)
        .bind(&metadata_json)
        .bind(is_final_i)
        .execute(&self.pool)
        .await?;
        Ok(id)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn list_artifacts(
        &self,
        project_id: Option<&str>,
        session_id: Option<&str>,
        kind: Option<&str>,
        exclude_kind: Option<&str>,
        trust_level: Option<&str>,
        unverified_only: bool,
        blocked_session_only: bool,
        final_only: bool,
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
        if exclude_kind.filter(|k| !k.is_empty()).is_some() {
            sql.push_str(" AND a.kind <> ?");
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
        if final_only {
            sql.push_str(" AND a.is_final = 1");
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
        if let Some(ek) = exclude_kind.filter(|k| !k.is_empty()) {
            q = q.bind(ek);
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
