use super::*;

impl DashboardDb {
    pub async fn list_recent_events(&self, limit: i64) -> Result<Vec<RecentEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT e.id, e.project_id, p.name AS project_name, e.session_id,
                   e.event_type, e.severity, e.title, e.occurred_at
            FROM project_events e
            JOIN projects p ON p.id = e.project_id
            ORDER BY e.occurred_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| RecentEvent {
                id: r.get("id"),
                project_id: r.get("project_id"),
                project_name: r.get("project_name"),
                session_id: r.get("session_id"),
                event_type: r.get("event_type"),
                severity: r.get("severity"),
                title: r.get("title"),
                occurred_at: r.get("occurred_at"),
            })
            .collect())
    }

    pub async fn insert_event(&self, req: InsertEventRequest) -> Result<ProjectEvent> {
        let id = format!("evt_{}", Uuid::new_v4().simple());
        let severity = req.severity.unwrap_or_else(|| "info".to_string());
        let body = req.body.unwrap_or_default();
        let payload = req.payload.unwrap_or(Value::Object(Default::default()));
        let payload_json = serde_json::to_string(&payload)?;
        sqlx::query(
            r#"
            INSERT INTO project_events
              (id, project_id, session_id, task_id, agent_id, event_type, severity, title, body, payload_json)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&req.project_id)
        .bind(&req.session_id)
        .bind(&req.task_id)
        .bind(&req.agent_id)
        .bind(&req.event_type)
        .bind(&severity)
        .bind(&req.title)
        .bind(&body)
        .bind(&payload_json)
        .execute(&self.pool)
        .await?;
        sqlx::query("UPDATE projects SET updated_at = datetime('now') WHERE id = ?")
            .bind(&req.project_id)
            .execute(&self.pool)
            .await?;
        self.get_event(&id)
            .await?
            .context("event missing after insert")
    }

    pub async fn get_event(&self, event_id: &str) -> Result<Option<ProjectEvent>> {
        let row = sqlx::query(
            r#"
            SELECT id, project_id, session_id, task_id, agent_id, event_type, severity,
                   title, body, payload_json, occurred_at
            FROM project_events WHERE id = ?
            "#,
        )
        .bind(event_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(row_to_event))
    }

    pub async fn list_session_events(
        &self,
        session_id: &str,
        after: Option<&str>,
        limit: i64,
        event_type: Option<&str>,
        severity: Option<&str>,
        q: Option<&str>,
    ) -> Result<Vec<ProjectEvent>> {
        let mut extra = String::new();
        if event_type.filter(|t| !t.is_empty()).is_some() {
            extra.push_str(" AND event_type = ?");
        }
        if severity.filter(|s| !s.is_empty()).is_some() {
            extra.push_str(" AND severity = ?");
        }
        if q.filter(|s| !s.is_empty()).is_some() {
            extra.push_str(" AND (title LIKE ? OR body LIKE ?)");
        }
        let rows = if let Some(after_id) = after {
            let occurred: Option<String> =
                sqlx::query_scalar("SELECT occurred_at FROM project_events WHERE id = ?")
                    .bind(after_id)
                    .fetch_optional(&self.pool)
                    .await?;
            let occurred = occurred.unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
            let sql = format!(
                r#"
                SELECT id, project_id, session_id, task_id, agent_id, event_type, severity,
                       title, body, payload_json, occurred_at
                FROM project_events
                WHERE session_id = ? AND occurred_at > ?{extra}
                ORDER BY occurred_at ASC
                LIMIT ?
                "#
            );
            let mut query = sqlx::query(&sql).bind(session_id).bind(occurred);
            if let Some(et) = event_type.filter(|t| !t.is_empty()) {
                query = query.bind(et);
            }
            if let Some(sev) = severity.filter(|s| !s.is_empty()) {
                query = query.bind(sev);
            }
            if let Some(text) = q.filter(|s| !s.is_empty()) {
                let pattern = format!("%{text}%");
                query = query.bind(pattern.clone()).bind(pattern);
            }
            query.bind(limit).fetch_all(&self.pool).await?
        } else {
            let sql = format!(
                r#"
                SELECT id, project_id, session_id, task_id, agent_id, event_type, severity,
                       title, body, payload_json, occurred_at
                FROM project_events
                WHERE session_id = ?{extra}
                ORDER BY occurred_at ASC
                LIMIT ?
                "#
            );
            let mut query = sqlx::query(&sql).bind(session_id);
            if let Some(et) = event_type.filter(|t| !t.is_empty()) {
                query = query.bind(et);
            }
            if let Some(sev) = severity.filter(|s| !s.is_empty()) {
                query = query.bind(sev);
            }
            if let Some(text) = q.filter(|s| !s.is_empty()) {
                let pattern = format!("%{text}%");
                query = query.bind(pattern.clone()).bind(pattern);
            }
            query.bind(limit).fetch_all(&self.pool).await?
        };
        Ok(rows.into_iter().map(row_to_event).collect())
    }

    pub async fn list_project_event_types(&self, project_id: &str) -> Result<Vec<String>> {
        let rows: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT event_type FROM project_events
            WHERE project_id = ?
            ORDER BY event_type
            "#,
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_session_event_types(&self, session_id: &str) -> Result<Vec<String>> {
        let rows: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT DISTINCT event_type FROM project_events
            WHERE session_id = ?
            ORDER BY event_type
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_project_events(
        &self,
        project_id: &str,
        limit: i64,
        event_type: Option<&str>,
        severity: Option<&str>,
        q: Option<&str>,
    ) -> Result<Vec<ProjectEvent>> {
        let mut extra = String::new();
        if event_type.filter(|t| !t.is_empty()).is_some() {
            extra.push_str(" AND event_type = ?");
        }
        if severity.filter(|s| !s.is_empty()).is_some() {
            extra.push_str(" AND severity = ?");
        }
        if q.filter(|s| !s.is_empty()).is_some() {
            extra.push_str(" AND (title LIKE ? OR body LIKE ?)");
        }
        let sql = format!(
            r#"
            SELECT id, project_id, session_id, task_id, agent_id, event_type, severity,
                   title, body, payload_json, occurred_at
            FROM project_events
            WHERE project_id = ?{extra}
            ORDER BY occurred_at DESC
            LIMIT ?
            "#
        );
        let mut query = sqlx::query(&sql).bind(project_id);
        if let Some(et) = event_type.filter(|t| !t.is_empty()) {
            query = query.bind(et);
        }
        if let Some(sev) = severity.filter(|s| !s.is_empty()) {
            query = query.bind(sev);
        }
        if let Some(text) = q.filter(|s| !s.is_empty()) {
            let pattern = format!("%{text}%");
            query = query.bind(pattern.clone()).bind(pattern);
        }
        let rows = query.bind(limit).fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(row_to_event).collect())
    }
}

fn row_to_event(r: sqlx::sqlite::SqliteRow) -> ProjectEvent {
    let payload_json: String = r.get("payload_json");
    let payload = serde_json::from_str(&payload_json).unwrap_or(Value::Null);
    ProjectEvent {
        id: r.get("id"),
        project_id: r.get("project_id"),
        session_id: r.get("session_id"),
        task_id: r.get("task_id"),
        agent_id: r.get("agent_id"),
        event_type: r.get("event_type"),
        severity: r.get("severity"),
        title: r.get("title"),
        body: r.get("body"),
        payload,
        occurred_at: r.get("occurred_at"),
    }
}
