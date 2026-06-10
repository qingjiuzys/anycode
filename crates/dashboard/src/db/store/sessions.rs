use super::*;

impl DashboardDb {
    pub async fn session_facets(&self) -> Result<SessionFacetsResponse> {
        let status = label_counts(
            &self,
            "SELECT status AS label, COUNT(*) AS cnt FROM sessions GROUP BY status ORDER BY cnt DESC",
        )
        .await?;
        let trusted_status = label_counts(
            &self,
            "SELECT trusted_status AS label, COUNT(*) AS cnt FROM sessions GROUP BY trusted_status ORDER BY cnt DESC",
        )
        .await?;
        let kind = label_counts(
            &self,
            "SELECT kind AS label, COUNT(*) AS cnt FROM sessions GROUP BY kind ORDER BY cnt DESC",
        )
        .await?;
        let pending_approval_total = crate::approval_ipc::pending_summary().pending_total as i64;
        let budget_exceeded_7d: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(DISTINCT session_id) FROM project_events
            WHERE event_type = 'budget_exceeded'
              AND occurred_at >= datetime('now', '-7 days')
            "#,
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        Ok(SessionFacetsResponse {
            status,
            trusted_status,
            kind,
            pending_approval_total,
            budget_exceeded_7d,
        })
    }
}

async fn label_counts(db: &DashboardDb, sql: &str) -> Result<Vec<LabelCount>> {
    let rows = sqlx::query(sql).fetch_all(&db.pool).await?;
    Ok(rows
        .into_iter()
        .map(|r| LabelCount {
            label: r.get("label"),
            count: r.get("cnt"),
        })
        .collect())
}

impl DashboardDb {
    pub async fn list_all_sessions(
        &self,
        limit: i64,
        kinds: Option<&[String]>,
        status: Option<&str>,
        trusted_status: Option<&str>,
        project_id: Option<&str>,
        budget_exceeded: bool,
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
        if budget_exceeded {
            conditions.push(
                "EXISTS (SELECT 1 FROM project_events e WHERE e.session_id = s.id AND e.event_type = 'budget_exceeded')".into(),
            );
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
                block_reason: None,
                block_kind: None,
            })
            .collect())
    }

    async fn enrich_session_with_project(
        &self,
        mut session: SessionWithProject,
    ) -> Result<SessionWithProject> {
        if session.trusted_status == "blocked"
            || session.status == "failed"
            || session.status == "pending"
        {
            let ctx = crate::db::resolve_block_context(
                self,
                &session.id,
                &session.status,
                &session.trusted_status,
                "",
            )
            .await?;
            session.block_reason = ctx.reason;
            session.block_kind = ctx.kind;
        }
        Ok(session)
    }

    async fn enrich_session_summary(&self, mut session: SessionSummary) -> Result<SessionSummary> {
        if session.trusted_status == "blocked"
            || session.status == "failed"
            || session.status == "pending"
        {
            let ctx = crate::db::resolve_block_context(
                self,
                &session.id,
                &session.status,
                &session.trusted_status,
                "",
            )
            .await?;
            session.block_reason = ctx.reason;
            session.block_kind = ctx.kind;
        }
        Ok(session)
    }

    async fn enrich_session_detail(&self, mut session: SessionDetail) -> Result<SessionDetail> {
        if session.trusted_status == "blocked"
            || session.status == "failed"
            || session.status == "pending"
        {
            let ctx = crate::db::resolve_block_context(
                self,
                &session.id,
                &session.status,
                &session.trusted_status,
                &session.summary,
            )
            .await?;
            session.block_reason = ctx.reason;
            session.block_kind = ctx.kind;
        }
        Ok(session)
    }

    pub async fn list_all_sessions_enriched(
        &self,
        limit: i64,
        kinds: Option<&[String]>,
        status: Option<&str>,
        trusted_status: Option<&str>,
        project_id: Option<&str>,
        budget_exceeded: bool,
    ) -> Result<Vec<SessionWithProject>> {
        let mut rows = self
            .list_all_sessions(
                limit,
                kinds,
                status,
                trusted_status,
                project_id,
                budget_exceeded,
            )
            .await?;
        for row in &mut rows {
            *row = self.enrich_session_with_project(row.clone()).await?;
        }
        Ok(rows)
    }

    pub async fn list_sessions_enriched(
        &self,
        project_id: &str,
        limit: i64,
    ) -> Result<Vec<SessionSummary>> {
        let mut rows = self.list_sessions(project_id, limit).await?;
        for row in &mut rows {
            *row = self.enrich_session_summary(row.clone()).await?;
        }
        Ok(rows)
    }

    pub async fn sweep_stale_pending_sessions(&self, max_age_minutes: i64) -> Result<u64> {
        let rows = sqlx::query(
            r#"
            SELECT id FROM sessions
            WHERE status = 'pending'
              AND (task_id IS NULL OR TRIM(task_id) = '')
              AND datetime(started_at) <= datetime('now', ?)
            "#,
        )
        .bind(format!("-{max_age_minutes} minutes"))
        .fetch_all(&self.pool)
        .await?;
        let mut updated = 0u64;
        for row in rows {
            let id: String = row.get("id");
            self.finish_session(
                &id,
                "failed",
                Some("Task did not start within timeout — check trigger logs or CLI availability."),
            )
            .await?;
            updated += 1;
        }
        Ok(updated)
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
                block_reason: None,
                block_kind: None,
            })
            .collect())
    }

    pub async fn get_session_enriched(&self, session_id: &str) -> Result<Option<SessionDetail>> {
        match self.get_session(session_id).await? {
            Some(session) => Ok(Some(self.enrich_session_detail(session).await?)),
            None => Ok(None),
        }
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
            block_reason: None,
            block_kind: None,
        }))
    }

    pub async fn get_session_by_task_id(&self, task_id: &str) -> Result<Option<SessionDetail>> {
        if task_id.trim().is_empty() {
            return Ok(None);
        }
        let row: Option<String> =
            sqlx::query_scalar("SELECT id FROM sessions WHERE task_id = ? LIMIT 1")
                .bind(task_id)
                .fetch_optional(&self.pool)
                .await?;
        match row {
            Some(id) => self.get_session(&id).await,
            None => Ok(None),
        }
    }

    pub async fn create_or_get_session_by_task_id(
        &self,
        req: CreateSessionRequest,
    ) -> Result<SessionDetail> {
        if let Some(task_id) = req.task_id.as_deref().filter(|t| !t.trim().is_empty()) {
            if let Some(existing) = self.get_session_by_task_id(task_id).await? {
                return Ok(existing);
            }
        }
        self.create_session(req).await
    }

    pub async fn create_session(&self, req: CreateSessionRequest) -> Result<SessionDetail> {
        self.create_session_with_status(req, "running").await
    }

    pub async fn create_planned_session(&self, req: CreateSessionRequest) -> Result<SessionDetail> {
        self.create_session_with_status(req, "pending").await
    }

    async fn create_session_with_status(
        &self,
        req: CreateSessionRequest,
        status: &str,
    ) -> Result<SessionDetail> {
        let id = format!("sess_{}", Uuid::new_v4().simple());
        let prompt_preview = req.prompt_preview.unwrap_or_default();
        let agent_type = req.agent_type.unwrap_or_default();
        let model = req.model.unwrap_or_default();
        let metadata_json = req.metadata_json.unwrap_or_else(|| "{}".into());
        sqlx::query(
            r#"
            INSERT INTO sessions (id, project_id, kind, task_id, title, prompt_preview, agent_type, model, metadata_json, status)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
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
        .bind(status)
        .execute(&self.pool)
        .await?;
        self.refresh_session_trusted_status(&id).await?;
        self.get_session(&id)
            .await?
            .context("session missing after create")
    }

    pub async fn update_session_agent(
        &self,
        session_id: &str,
        agent_type: Option<&str>,
    ) -> Result<()> {
        let Some(agent) = agent_type.map(str::trim).filter(|s| !s.is_empty()) else {
            return Ok(());
        };
        sqlx::query(
            r#"
            UPDATE sessions SET agent_type = ? WHERE id = ?
            "#,
        )
        .bind(agent)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn attach_task_to_session(
        &self,
        session_id: &str,
        task_id: &str,
        agent_type: Option<&str>,
        prompt_preview: Option<&str>,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE sessions
            SET task_id = ?,
                status = 'running',
                ended_at = NULL,
                agent_type = CASE WHEN ? IS NOT NULL AND TRIM(?) != '' THEN ? ELSE agent_type END,
                prompt_preview = CASE WHEN ? IS NOT NULL AND TRIM(?) != '' THEN ? ELSE prompt_preview END
            WHERE id = ?
            "#,
        )
        .bind(task_id)
        .bind(agent_type)
        .bind(agent_type)
        .bind(agent_type)
        .bind(prompt_preview)
        .bind(prompt_preview)
        .bind(prompt_preview)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        self.refresh_session_trusted_status(session_id).await?;
        Ok(())
    }

    pub async fn update_session_metadata(
        &self,
        session_id: &str,
        title: Option<&str>,
        prompt_preview: Option<&str>,
    ) -> Result<()> {
        if title.is_none() && prompt_preview.is_none() {
            return Ok(());
        }
        sqlx::query(
            r#"
            UPDATE sessions
            SET title = COALESCE(?, title),
                prompt_preview = COALESCE(?, prompt_preview)
            WHERE id = ?
            "#,
        )
        .bind(title)
        .bind(prompt_preview)
        .bind(session_id)
        .execute(&self.pool)
        .await?;
        Ok(())
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
        if let Some(sess) = self.get_session(session_id).await? {
            self.refresh_project_trust_score(&sess.project_id).await?;
            if status == crate::db::trusted::TrustedStatus::Blocked {
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
                block_reason: None,
                block_kind: None,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::CreateSessionRequest;

    #[tokio::test]
    async fn create_or_get_session_by_task_id_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let db = DashboardDb::open(dir.path().join("sessions.db"))
            .await
            .unwrap();
        let project = db
            .upsert_project(UpsertProjectRequest {
                root_path: "/tmp/idempotent".into(),
                name: Some("demo".into()),
                description: None,
                create_root: None,
                ..Default::default()
            })
            .await
            .unwrap();
        let req = CreateSessionRequest {
            project_id: project.id,
            kind: "run".into(),
            task_id: Some("task-abc".into()),
            title: "first title".into(),
            prompt_preview: Some("hello".into()),
            agent_type: Some("general".into()),
            model: None,
            metadata_json: None,
        };
        let first = db
            .create_or_get_session_by_task_id(req.clone())
            .await
            .unwrap();
        let second = db.create_or_get_session_by_task_id(req).await.unwrap();
        assert_eq!(first.id, second.id);
        assert_eq!(second.title, "first title");
    }
}
