use super::sqlite_shared::{discover_sqlite_files, open_readonly, query_message_tables};
use crate::backend::WechatHistoryBackend;
use crate::config::WechatHistoryConfig;
use crate::date_filter::{filter_messages, validate_query};
use crate::format::render_markdown_table;
use crate::model::{WechatHistoryQuery, WechatHistoryResult};
use crate::{Result, WechatHistoryError};
use std::path::PathBuf;

pub struct SqlitePlainBackend {
    data_dir: PathBuf,
}

impl SqlitePlainBackend {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }
}

impl WechatHistoryBackend for SqlitePlainBackend {
    fn name(&self) -> &'static str {
        "sqlite_plain"
    }

    fn query(
        &self,
        query: &WechatHistoryQuery,
        config: &WechatHistoryConfig,
    ) -> Result<WechatHistoryResult> {
        if !self.data_dir.exists() {
            return Err(WechatHistoryError::BackendNotConfigured(format!(
                "dataDir not found: {}",
                self.data_dir.display()
            )));
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| WechatHistoryError::Sqlite(format!("tokio runtime: {e}")))?;
        rt.block_on(self.query_async(query, config))
    }
}

impl SqlitePlainBackend {
    async fn query_async(
        &self,
        query: &WechatHistoryQuery,
        config: &WechatHistoryConfig,
    ) -> Result<WechatHistoryResult> {
        let (date, tz, start_ms, end_ms) = validate_query(query)?;
        let limit = config.effective_limit(query.limit);
        let talker = query.conversation.as_deref();
        let mut messages = Vec::new();
        for db in discover_sqlite_files(&self.data_dir) {
            let pool = open_readonly(&db).await?;
            messages.extend(query_message_tables(&pool, talker).await?);
        }
        let (messages, truncated) = filter_messages(messages, query, start_ms, end_ms, limit);
        let markdown_table = render_markdown_table(&messages, query, tz);
        Ok(WechatHistoryResult {
            date: date.format("%Y-%m-%d").to_string(),
            timezone: tz.to_string(),
            backend: self.name().into(),
            total: messages.len(),
            truncated,
            markdown_table: Some(markdown_table),
            messages,
            attachment_stats: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{MessageDirection, WechatHistoryQuery};
    use sqlx::sqlite::SqliteConnectOptions;
    use std::str::FromStr;
    use tempfile::TempDir;

    async fn seed_db(path: &std::path::Path, create_time: i64) {
        let opts = SqliteConnectOptions::from_str(&format!("sqlite:{}", path.display()))
            .unwrap()
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await
            .unwrap();
        sqlx::query(
            "CREATE TABLE Message (
                MsgSvrID TEXT,
                Type INTEGER,
                CreateTime INTEGER,
                StrTalker TEXT,
                StrContent TEXT,
                IsSender INTEGER,
                Des TEXT
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO Message VALUES ('1', 1, ?1, 'wxid_alice', 'hello sqlite', 0, 'Alice')",
        )
        .bind(create_time)
        .execute(&pool)
        .await
        .unwrap();
    }

    #[test]
    fn sqlite_plain_reads_message_table() {
        let dir = TempDir::new().unwrap();
        let db = dir.path().join("message_0.db");
        let query = WechatHistoryQuery {
            date: "2026-06-14".into(),
            conversation: Some("alice".into()),
            keyword: None,
            timezone: Some("Asia/Shanghai".into()),
            limit: None,
            include_group_sender: true,
            ..Default::default()
        };
        let (_, _, start_ms, _) = crate::date_filter::validate_query(&query).unwrap();
        let create_time = start_ms / 1000 + 3600;
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(seed_db(&db, create_time));
        let backend = SqlitePlainBackend::new(dir.path().to_path_buf());
        let config = WechatHistoryConfig {
            enabled: true,
            backend: crate::config::WechatHistoryBackendKind::SqlitePlain,
            ..Default::default()
        };
        let result = backend.query(&query, &config).unwrap();
        assert_eq!(result.total, 1);
        assert_eq!(result.messages[0].direction, MessageDirection::Inbound);
    }
}
