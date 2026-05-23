use super::*;

impl DashboardDb {
    pub async fn upsert_local_service(
        &self,
        name: &str,
        host: &str,
        port: u16,
        status: &str,
        auth_mode: &str,
        pid: Option<u32>,
    ) -> Result<()> {
        let id = format!("svc_{name}_{host}_{port}");
        sqlx::query(
            r#"
            INSERT INTO local_services (id, name, host, port, status, auth_mode, pid, started_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
            ON CONFLICT(name, host, port) DO UPDATE SET
              status = excluded.status,
              auth_mode = excluded.auth_mode,
              pid = excluded.pid,
              started_at = CASE WHEN excluded.status = 'running' THEN datetime('now') ELSE local_services.started_at END,
              updated_at = datetime('now')
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(host)
        .bind(i64::from(port))
        .bind(status)
        .bind(auth_mode)
        .bind(pid.map(i64::from))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_local_services(&self) -> Result<Vec<(String, String, i64, String, String)>> {
        let rows = sqlx::query(
            "SELECT name, host, port, status, auth_mode FROM local_services ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get("name"),
                    r.get("host"),
                    r.get("port"),
                    r.get("status"),
                    r.get("auth_mode"),
                )
            })
            .collect())
    }
}
