use anyhow::{Context, Result};
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use std::path::Path;

#[derive(Clone)]
pub struct AccountDb {
    pool: MySqlPool,
}

impl AccountDb {
    pub async fn connect(database_url: &str) -> Result<Self> {
        let mut last_err = None;
        for attempt in 0..5 {
            match MySqlPoolOptions::new()
                .max_connections(10)
                .connect(database_url)
                .await
            {
                Ok(pool) => return Ok(Self { pool }),
                Err(error) => {
                    last_err = Some(error);
                    if attempt < 4 {
                        tokio::time::sleep(std::time::Duration::from_millis(
                            500 * (attempt + 1) as u64,
                        ))
                        .await;
                    }
                }
            }
        }
        Err(last_err.unwrap()).context("connect to MySQL")
    }

    /// Applies additive MySQL migrations after the base deployment schema.
    pub async fn migrate(&self) -> Result<()> {
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS account_schema_migrations (version VARCHAR(64) NOT NULL PRIMARY KEY, applied_at DATETIME(3) NOT NULL DEFAULT CURRENT_TIMESTAMP(3))",
        )
        .execute(&self.pool)
        .await?;
        self.apply_migration(
            "009_account_identity_and_sessions",
            include_str!("../migrations/009_account_identity_and_sessions.sql"),
        )
        .await?;
        self.apply_migration(
            "010_conversation_audit",
            include_str!("../migrations/010_conversation_audit.sql"),
        )
        .await?;
        self.apply_migration(
            "011_cny_billing",
            include_str!("../migrations/011_cny_billing.sql"),
        )
        .await?;
        self.apply_migration(
            "012_agnes_upstream_model",
            include_str!("../migrations/012_agnes_upstream_model.sql"),
        )
        .await?;
        self.apply_migration(
            "013_deepseek_cloud_models",
            include_str!("../migrations/013_deepseek_cloud_models.sql"),
        )
        .await?;
        self.apply_migration(
            "014_cloud_plans",
            include_str!("../migrations/014_cloud_plans.sql"),
        )
        .await?;
        self.apply_migration(
            "015_memory_sync",
            include_str!("../migrations/015_memory_sync.sql"),
        )
        .await?;
        self.apply_migration(
            "016_a2a_handoff",
            include_str!("../migrations/016_a2a_handoff.sql"),
        )
        .await?;
        self.apply_migration(
            "017_a2a_stream_token_ephemeral",
            include_str!("../migrations/017_a2a_stream_token_ephemeral.sql"),
        )
        .await?;
        Ok(())
    }

    async fn apply_migration(&self, version: &str, sql: &str) -> Result<()> {
        let applied: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM account_schema_migrations WHERE version = ?")
                .bind(version)
                .fetch_one(&self.pool)
                .await?;
        if applied > 0 {
            return Ok(());
        }
        // Strip `--` line comments before splitting on `;` so comments cannot
        // inject stray statement fragments (e.g. "stored; this is only…").
        let without_line_comments: String = sql
            .lines()
            .filter(|line| {
                let t = line.trim();
                !t.is_empty() && !t.starts_with("--")
            })
            .collect::<Vec<_>>()
            .join("\n");
        for statement in without_line_comments
            .split(';')
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Self::execute_with_retry(&self.pool, statement, version).await?;
        }
        sqlx::query("INSERT INTO account_schema_migrations (version) VALUES (?)")
            .bind(version)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn execute_with_retry(pool: &MySqlPool, statement: &str, version: &str) -> Result<()> {
        let mut last_err: Option<sqlx::Error> = None;
        for attempt in 0..3 {
            match sqlx::query(statement).execute(pool).await {
                Ok(_) => return Ok(()),
                Err(error) if is_duplicate_schema_error(&error) => return Ok(()),
                Err(error) => {
                    last_err = Some(error);
                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_millis(400 * (attempt + 1)))
                            .await;
                    }
                }
            }
        }
        Err(last_err.unwrap()).with_context(|| format!("apply migration {version}"))
    }

    pub fn pool(&self) -> &MySqlPool {
        &self.pool
    }
}

impl std::fmt::Debug for AccountDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountDb").finish_non_exhaustive()
    }
}

#[allow(dead_code)]
pub fn migration_dir() -> &'static Path {
    Path::new("./migrations")
}

fn is_duplicate_schema_error(error: &sqlx::Error) -> bool {
    if let Some(db) = error.as_database_error() {
        if matches!(
            db.code().as_deref(),
            Some("1060" | "1061" | "1062" | "1050" | "42S21" | "42S01")
        ) {
            return true;
        }
    }
    let msg = error.to_string();
    msg.contains("Duplicate column")
        || msg.contains("Duplicate key name")
        || msg.contains("already exists")
}
