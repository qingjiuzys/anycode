use super::*;

impl DashboardDb {
    pub async fn list_all_sessions(
        &self,
        limit: i64,
        kinds: Option<&[String]>,
        status: Option<&str>,
        trusted_status: Option<&str>,
        project_id: Option<&str>,
    ) -> Result<Vec<SessionWithProject>> {
        if kinds.is_some_and(|k| k.is_empty()) {
            return Ok(vec![]);
        }
        let mut conditions = vec!["1=1".to_string()];
        if kinds.is_some() {
            conditions.push(format!(
                "s.kind IN ({})",
                kinds
                    .unwrap()
                    .iter()
                    .map(|_| "?")
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        if status.filter(|s| !s.is_empty()).is_some() {
            conditions.push("s.status = ?".into());
        }
        if trusted_status.filter(|s| !s.is_empty()).is_some() {
            conditions.push("s.trusted_status = ?".into());
        }
        if project_id.filter(|s| !s.is_empty()).is_some() {
            conditions.push("s.project_id = ?".into());
        }
        let where_clause = conditions.join(" AND ");
        let sql = format!(
            r#"
            SELECT s.id, s.project_id, p.name AS project_name, s.kind, s.task_id, s.title,
                   s.status, s.trusted_status, s.agent_type, s.model, s.started_at, s.ended_at
            FROM sessions s
            JOIN projects p ON p.id = s.project_id
            WHERE {where_clause}
            ORDER BY s.started_at DESC
            LIMIT ?
            "#
        );
        let mut q = sqlx::query(&sql);
        if let Some(kinds) = kinds {
            for k in kinds {
                q = q.bind(k);
            }
        }
        if let Some(st) = status.filter(|s| !s.is_empty()) {
            q = q.bind(st);
        }
        if let Some(ts) = trusted_status.filter(|s| !s.is_empty()) {
            q = q.bind(ts);
        }
        if let Some(pid) = project_id.filter(|s| !s.is_empty()) {
            q = q.bind(pid);
        }
        let rows = q.bind(limit).fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|r| SessionWithProject {
                id: r.get("id"),
                project_id: r.get("project_id"),
                project_name: r.get("project_name"),
                kind: r.get("kind"),
                task_id: r.get("task_id"),
                title: r.get("title"),
                status: r.get("status"),
                trusted_status: r.get("trusted_status"),
                agent_type: r.get("agent_type"),
                model: r.get("model"),
                started_at: r.get("started_at"),
                ended_at: r.get("ended_at"),
            })
            .collect())
    }

    pub async fn list_sessions(&self, project_id: &str, limit: i64) -> Result<Vec<SessionSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, task_id, title, status, trusted_status, agent_type, model,
                   started_at, ended_at
            FROM sessions
            WHERE project_id = ?
            ORDER BY started_at DESC
            LIMIT ?
            "#,
        )
        .bind(project_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| SessionSummary {
                id: r.get("id"),
                kind: r.get("kind"),
                task_id: r.get("task_id"),
                title: r.get("title"),
                status: r.get("status"),
                trusted_status: r.get("trusted_status"),
                agent_type: r.get("agent_type"),
                model: r.get("model"),
                started_at: r.get("started_at"),
                ended_at: r.get("ended_at"),
            })
            .collect())
    }

    pub async fn get_session(&self, session_id: &str) -> Result<Option<SessionDetail>> {
        let row = sqlx::query(
            r#"
            SELECT s.id, s.project_id, p.name AS project_name, s.kind, s.task_id, s.title,
                   s.prompt_preview, s.status, s.trusted_status, s.agent_type, s.model,
                   s.started_at, s.ended_at, s.summary, s.metadata_json
            FROM sessions s
            JOIN projects p ON p.id = s.project_id
            WHERE s.id = ?
            "#,
        )
        .bind(session_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| SessionDetail {
            id: r.get("id"),
            project_id: r.get("project_id"),
            project_name: r.get("project_name"),
            kind: r.get("kind"),
            task_id: r.get("task_id"),
            title: r.get("title"),
            prompt_preview: r.get("prompt_preview"),
            status: r.get("status"),
            trusted_status: r.get("trusted_status"),
            agent_type: r.get("agent_type"),
            model: r.get("model"),
            started_at: r.get("started_at"),
            ended_at: r.get("ended_at"),
            summary: r.get("summary"),
            metadata_json: r.get("metadata_json"),
        }))
    }

    pub async fn create_session(&self, req: CreateSessionRequest) -> Result<SessionDetail> {
        let id = format!("sess_{}", Uuid::new_v4().simple());
        let prompt_preview = req.prompt_preview.unwrap_or_default();
        let agent_type = req.agent_type.unwrap_or_default();
        let model = req.model.unwrap_or_default();
        let metadata_json = req.metadata_json.unwrap_or_else(|| "{}".into());
        sqlx::query(
            r#"
            INSERT INTO sessions (id, project_id, kind, task_id, title, prompt_preview, agent_type, model, metadata_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&req.project_id)
        .bind(&req.kind)
        .bind(&req.task_id)
        .bind(&req.title)
        .bind(&prompt_preview)
        .bind(&agent_type)
        .bind(&model)
        .bind(&metadata_json)
        .execute(&self.pool)
        .await?;
        self.refresh_session_trusted_status(&id).await?;
        self.get_session(&id)
            .await?
            .context("session missing after create")
    }

    pub async fn finish_session(
        &self,
        session_id: &str,
        status: &str,
        summary: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET status = ?, ended_at = datetime('now'), summary = COALESCE(?, summary)
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(summary)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        self.refresh_session_trusted_status(session_id).await?;
        if let Some(sess) = self.get_session(session_id).await? {
            let _ = crate::automation_policy::handle_session_completed(
                self,
                &sess.project_id,
                session_id,
                &sess.status,
                &sess.trusted_status,
            )
            .await;
        }
        Ok(())
    }

    /// Mark a running session cancelled (dashboard control plane; does not kill CLI process).
    pub async fn cancel_running_session(&self, session_id: &str) -> Result<bool> {
        let result = sqlx::query(
            r#"
            UPDATE sessions
            SET status = 'cancelled',
                ended_at = datetime('now'),
                summary = CASE
                  WHEN summary IS NULL OR TRIM(summary) = '' THEN 'Cancelled from dashboard'
                  ELSE summary
                END
            WHERE id = ? AND status = 'running'
            "#,
        )
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        if result.rows_affected() == 0 {
            return Ok(false);
        }
        self.refresh_session_trusted_status(session_id).await?;
        if let Some(sess) = self.get_session(session_id).await? {
            let _ = crate::automation_policy::handle_session_completed(
                self,
                &sess.project_id,
                session_id,
                &sess.status,
                &sess.trusted_status,
            )
            .await;
        }
        Ok(true)
    }

    pub async fn update_session_model(&self, session_id: &str, model: &str) -> Result<()> {
        sqlx::query("UPDATE sessions SET model = ? WHERE id = ? AND (model = '' OR model IS NULL)")
            .bind(model)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn refresh_session_trusted_status(&self, session_id: &str) -> Result<()> {
        let counts: (i64, i64, i64) = sqlx::query_as(
            r#"
            SELECT
              COUNT(*) FILTER (WHERE required = 1),
              COUNT(*) FILTER (WHERE required = 1 AND status = 'failed'),
              COUNT(*) FILTER (WHERE required = 1 AND status IN ('pending', 'running'))
            FROM gates WHERE session_id = ?
            "#,
        )
        .bind(session_id)
        .fetch_one(&self.pool)
        .await?;
        let session_status = self.get_session(session_id).await?.map(|s| s.status);
        let status =
            compute_trusted_status(counts.0, counts.1, counts.2, session_status.as_deref());
        sqlx::query("UPDATE sessions SET trusted_status = ? WHERE id = ?")
            .bind(status.as_str())
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        self.sync_session_artifact_trust(session_id, status).await?;
        if status == crate::db::trusted::TrustedStatus::Blocked {
            if let Some(sess) = self.get_session(session_id).await? {
                let _ = crate::automation_policy::handle_trust_blocked(
                    self,
                    &sess.project_id,
                    session_id,
                    "blocked",
                )
                .await;
            }
        }
        Ok(())
    }

    async fn sync_session_artifact_trust(
        &self,
        session_id: &str,
        status: crate::db::trusted::TrustedStatus,
    ) -> Result<()> {
        use crate::db::trusted::TrustedStatus;
        if status != TrustedStatus::Verified {
            return Ok(());
        }
        let gates = self.list_gates_for_session(session_id).await?;
        let verifying_gate = gates.iter().find(|g| g.required && g.status == "passed");
        let Some(gate) = verifying_gate else {
            return Ok(());
        };
        sqlx::query(
            r#"
            UPDATE artifacts
            SET trust_level = 'verified', verified_by_gate_id = ?, updated_at = datetime('now')
            WHERE session_id = ?
              AND kind != 'report'
              AND trust_level IN ('needs_verify', 'unknown', 'unverified')
            "#,
        )
        .bind(&gate.id)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn merge_session_metadata(&self, session_id: &str, patch: &Value) -> Result<()> {
        let row: Option<String> =
            sqlx::query_scalar("SELECT metadata_json FROM sessions WHERE id = ?")
                .bind(session_id)
                .fetch_optional(&self.pool)
                .await?;
        let Some(raw) = row else {
            anyhow::bail!("session not found");
        };
        let mut meta: Value =
            serde_json::from_str(&raw).unwrap_or(Value::Object(Default::default()));
        if let (Value::Object(ref mut base), Value::Object(extra)) = (&mut meta, patch) {
            for (k, v) in extra {
                base.insert(k.clone(), v.clone());
            }
        }
        sqlx::query("UPDATE sessions SET metadata_json = ? WHERE id = ?")
            .bind(serde_json::to_string(&meta)?)
            .bind(session_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn find_session_by_correlation(
        &self,
        correlation_id: &str,
    ) -> Result<Option<String>> {
        if correlation_id.is_empty() {
            return Ok(None);
        }
        let row: Option<String> = sqlx::query_scalar(
            r#"
            SELECT id FROM sessions
            WHERE json_extract(metadata_json, '$.correlation_id') = ?
            ORDER BY started_at DESC
            LIMIT 1
            "#,
        )
        .bind(correlation_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn list_agent_usage_stats(&self, limit: i64) -> Result<Vec<AgentUsageStat>> {
        let rows = sqlx::query(
            r#"
            SELECT agent_type, model, COUNT(*) AS cnt, MAX(started_at) AS last_at
            FROM sessions
            WHERE agent_type != ''
            GROUP BY agent_type, model
            ORDER BY cnt DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| AgentUsageStat {
                agent_type: r.get("agent_type"),
                model: r.get("model"),
                sessions_count: r.get("cnt"),
                last_started_at: r.get("last_at"),
            })
            .collect())
    }

    pub async fn list_running_sessions(&self, limit: i64) -> Result<Vec<SessionWithProject>> {
        let rows = sqlx::query(
            r#"
            SELECT s.id, s.project_id, p.name AS project_name, s.kind, s.task_id, s.title,
                   s.status, s.trusted_status, s.agent_type, s.model, s.started_at, s.ended_at
            FROM sessions s
            JOIN projects p ON p.id = s.project_id
            WHERE s.status = 'running'
            ORDER BY s.started_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| SessionWithProject {
                id: r.get("id"),
                project_id: r.get("project_id"),
                project_name: r.get("project_name"),
                kind: r.get("kind"),
                task_id: r.get("task_id"),
                title: r.get("title"),
                status: r.get("status"),
                trusted_status: r.get("trusted_status"),
                agent_type: r.get("agent_type"),
                model: r.get("model"),
                started_at: r.get("started_at"),
                ended_at: r.get("ended_at"),
            })
            .collect())
    }
}
