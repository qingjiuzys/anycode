use super::*;

impl DashboardDb {
    pub async fn upsert_gate(
        &self,
        project_id: &str,
        session_id: &str,
        name: &str,
        command: &str,
        status: &str,
        required: bool,
        output_excerpt: &str,
    ) -> Result<String> {
        let id = format!("gate_{}_{}", session_id, name.replace(' ', "_"));
        let req = i64::from(required);
        sqlx::query(
            r#"
            INSERT INTO gates (id, project_id, session_id, name, command, status, required, output_excerpt, started_at, ended_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
              status = excluded.status,
              output_excerpt = excluded.output_excerpt,
              ended_at = datetime('now')
            "#,
        )
        .bind(&id)
        .bind(project_id)
        .bind(session_id)
        .bind(name)
        .bind(command)
        .bind(status)
        .bind(req)
        .bind(output_excerpt)
        .execute(&self.pool)
        .await?;
        self.refresh_session_trusted_status(session_id).await?;
        let _ = crate::automation_policy::handle_gate_outcome(
            self, project_id, session_id, name, status, required,
        )
        .await;
        Ok(id)
    }

    /// Stable session for manual gate runs from the dashboard UI.
    pub async fn ensure_manual_gate_session(&self, project_id: &str) -> Result<String> {
        if let Some(id) = sqlx::query_scalar::<_, String>(
            "SELECT id FROM sessions WHERE project_id = ? AND kind = 'manual_gate' LIMIT 1",
        )
        .bind(project_id)
        .fetch_optional(&self.pool)
        .await?
        {
            return Ok(id);
        }
        let session = self
            .create_session(CreateSessionRequest {
                project_id: project_id.to_string(),
                kind: "manual_gate".into(),
                task_id: None,
                title: "Manual gate verification".into(),
                prompt_preview: None,
                agent_type: None,
                model: None,
                metadata_json: None,
            })
            .await?;
        Ok(session.id)
    }

    pub async fn list_gates_for_project(&self, project_id: &str) -> Result<Vec<GateRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, status, required, output_excerpt, session_id
            FROM gates WHERE project_id = ?
            ORDER BY COALESCE(ended_at, started_at) DESC
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_gate).collect())
    }

    pub async fn list_gates_for_session(&self, session_id: &str) -> Result<Vec<GateRecord>> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, status, required, output_excerpt, session_id
            FROM gates WHERE session_id = ?
            ORDER BY COALESCE(ended_at, started_at) DESC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(row_to_gate).collect())
    }
}

fn row_to_gate(r: sqlx::sqlite::SqliteRow) -> GateRecord {
    let required: i64 = r.get("required");
    GateRecord {
        id: r.get("id"),
        name: r.get("name"),
        status: r.get("status"),
        required: required != 0,
        output_excerpt: r.get("output_excerpt"),
        session_id: r.get("session_id"),
    }
}
