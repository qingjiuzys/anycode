use super::DashboardDb;
use crate::schema::{LOCAL_ORG_ID, LOCAL_USER_ID};
use anyhow::{Context, Result};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

impl DashboardDb {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("create db parent {}", parent.display()))?;
        }
        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))?
            .create_if_missing(true)
            .foreign_keys(true)
            .busy_timeout(Duration::from_secs(5));
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let db = Self { pool, path };
        db.ensure_local_org().await?;
        Ok(db)
    }

    async fn ensure_local_org(&self) -> Result<()> {
        let org_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM organizations WHERE id = ?")
            .bind(LOCAL_ORG_ID)
            .fetch_one(&self.pool)
            .await?;
        if org_exists == 0 {
            sqlx::query("INSERT INTO organizations (id, name, mode) VALUES (?, 'Local', 'local')")
                .bind(LOCAL_ORG_ID)
                .execute(&self.pool)
                .await?;
            sqlx::query(
                "INSERT INTO users (id, organization_id, email, display_name, role)
                 VALUES (?, ?, 'local@anycode', 'Local User', 'owner')",
            )
            .bind(LOCAL_USER_ID)
            .bind(LOCAL_ORG_ID)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }
}
