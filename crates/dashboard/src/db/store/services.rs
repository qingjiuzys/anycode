use super::*;

#[derive(Debug, Clone)]
pub struct LocalServiceRow {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub status: String,
    pub auth_mode: String,
    pub pid: Option<u32>,
}

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

    pub async fn mark_local_service_stopped(
        &self,
        name: &str,
        host: &str,
        port: u16,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE local_services
            SET status = 'stopped', pid = NULL, updated_at = datetime('now')
            WHERE name = ? AND host = ? AND port = ?
            "#,
        )
        .bind(name)
        .bind(host)
        .bind(i64::from(port))
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_local_services_with_pid(&self) -> Result<Vec<LocalServiceRow>> {
        let rows = sqlx::query(
            "SELECT name, host, port, status, auth_mode, pid FROM local_services ORDER BY name, port",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .filter_map(|r| {
                let port_i: i64 = r.get("port");
                if port_i < 0 || port_i > i64::from(u16::MAX) {
                    return None;
                }
                let pid_i: Option<i64> = r.get("pid");
                Some(LocalServiceRow {
                    name: r.get("name"),
                    host: r.get("host"),
                    port: port_i as u16,
                    status: r.get("status"),
                    auth_mode: r.get("auth_mode"),
                    pid: pid_i.and_then(|p| u32::try_from(p).ok()),
                })
            })
            .collect())
    }

    pub async fn list_running_local_services(&self, name: &str) -> Result<Vec<LocalServiceRow>> {
        Ok(self
            .list_local_services_with_pid()
            .await?
            .into_iter()
            .filter(|row| row.name == name && row.status == "running")
            .collect())
    }

    pub async fn reconcile_local_services(&self, name: &str) -> Result<u64> {
        let rows = self.list_running_local_services(name).await?;
        let mut updated = 0u64;
        for row in rows {
            if !crate::service_governance::is_dashboard_service_live(&row.host, row.port, row.pid)
                .await
            {
                self.mark_local_service_stopped(&row.name, &row.host, row.port)
                    .await?;
                updated += 1;
            }
        }
        Ok(updated)
    }

    pub async fn list_local_services(&self) -> Result<Vec<(String, String, i64, String, String)>> {
        let rows = sqlx::query(
            "SELECT name, host, port, status, auth_mode FROM local_services ORDER BY name, port",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reconcile_marks_stale_running_as_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let db_path = dir.path().join("projects.db");
        let db = DashboardDb::open(&db_path).await.unwrap();
        db.upsert_local_service(
            "dashboard",
            "127.0.0.1",
            59998,
            "running",
            "local",
            Some(9_999_999),
        )
        .await
        .unwrap();

        let updated = db.reconcile_local_services("dashboard").await.unwrap();
        assert_eq!(updated, 1);

        let rows = db.list_local_services().await.unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].3, "stopped");
    }
}
