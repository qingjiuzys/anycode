use super::sqlite_shared::chatlog_message_from_value;
use crate::backend::WechatHistoryBackend;
use crate::config::WechatHistoryConfig;
use crate::date_filter::{filter_messages, validate_query};
use crate::format::render_markdown_table;
use crate::model::{WechatHistoryQuery, WechatHistoryResult};
use crate::{Result, WechatHistoryError};
use reqwest::Client;
use std::time::Duration;

pub struct ChatlogHttpBackend {
    endpoint: String,
    client: Client,
}

impl ChatlogHttpBackend {
    pub fn new(endpoint: String) -> Self {
        Self {
            endpoint: endpoint.trim_end_matches('/').to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("reqwest client"),
        }
    }
}

impl WechatHistoryBackend for ChatlogHttpBackend {
    fn name(&self) -> &'static str {
        "chatlog_http"
    }

    fn query(
        &self,
        query: &WechatHistoryQuery,
        config: &WechatHistoryConfig,
    ) -> Result<WechatHistoryResult> {
        crate::runtime::block_on_async(self.query_async(query, config))
    }
}

impl ChatlogHttpBackend {
    async fn query_async(
        &self,
        query: &WechatHistoryQuery,
        config: &WechatHistoryConfig,
    ) -> Result<WechatHistoryResult> {
        let (date, tz, start_ms, end_ms) = validate_query(query)?;
        let limit = config.effective_limit(query.limit);
        let url = format!(
            "{}/api/v1/chatlog?time={}&format=json&limit={}",
            self.endpoint,
            urlencoding::encode(query.date.trim()),
            limit
        );
        let url = if let Some(conv) = query
            .conversation
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            format!("{}&talker={}", url, urlencoding::encode(conv))
        } else {
            url
        };

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| WechatHistoryError::Http(format!("GET {url}: {e}")))?;
        if !resp.status().is_success() {
            return Err(WechatHistoryError::Http(format!(
                "GET {url} returned {}",
                resp.status()
            )));
        }
        let body: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| WechatHistoryError::Http(format!("decode JSON from {url}: {e}")))?;

        let messages = extract_chatlog_messages(&body);
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

fn extract_chatlog_messages(body: &serde_json::Value) -> Vec<crate::model::WechatChatMessage> {
    let items = body
        .get("items")
        .or_else(|| body.get("data"))
        .or_else(|| body.get("messages"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_else(|| {
            if body.is_array() {
                body.as_array().cloned().unwrap_or_default()
            } else {
                vec![]
            }
        });
    items
        .iter()
        .filter_map(chatlog_message_from_value)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WechatHistoryQuery;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn chatlog_http_fetches_and_filters_by_date() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/chatlog"))
            .and(query_param("time", "2026-06-14"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    {
                        "time": "2026-06-14 10:00:00",
                        "talker": "wxid_a",
                        "talkerName": "Alice",
                        "content": "morning",
                        "type": 1,
                        "isSend": false
                    }
                ]
            })))
            .mount(&server)
            .await;

        let backend = ChatlogHttpBackend::new(server.uri());
        let config = WechatHistoryConfig {
            enabled: true,
            ..Default::default()
        };
        let query = WechatHistoryQuery {
            date: "2026-06-14".into(),
            conversation: None,
            keyword: None,
            timezone: Some("Asia/Shanghai".into()),
            limit: None,
            include_group_sender: true,
            ..Default::default()
        };
        let result = backend.query_async(&query, &config).await.unwrap();
        assert_eq!(result.total, 1);
        assert!(result.messages[0].content.contains("morning"));
    }
}
