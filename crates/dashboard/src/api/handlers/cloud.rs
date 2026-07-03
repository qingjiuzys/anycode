use super::*;
use anycode_llm::{default_gateway_chat_url, read_cloud_access_token, resolve_gateway_host};
use serde::Serialize;

#[derive(Serialize)]
pub struct CloudLinkStartResponse {
    pub device_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_code: Option<String>,
    pub verification_uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_uri_complete: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<u64>,
    pub browser_url: String,
    pub redirect_uri: &'static str,
}

pub async fn post_cloud_link_start() -> impl IntoResponse {
    match anycode_setup::start_device_link().await {
        Ok(start) => {
            let browser_url = anycode_setup::browser_url_for_device_link(&start);
            Json(CloudLinkStartResponse {
                device_code: start.device_code,
                user_code: start.user_code,
                verification_uri: start.verification_uri,
                verification_uri_complete: start.verification_uri_complete,
                expires_in: start.expires_in,
                browser_url,
                redirect_uri: anycode_setup::DEVICE_LINK_REDIRECT_URI,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
pub struct CloudSessionResponse {
    pub linked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway_url: Option<String>,
}

pub async fn get_cloud_session() -> Json<CloudSessionResponse> {
    let portal_url = Some(anycode_llm::cloud_portal_url());
    let gateway_url = Some(anycode_llm::resolve_gateway_host());
    let path = anycode_llm::cloud_session_path();
    if !path.is_file() {
        return Json(CloudSessionResponse {
            linked: false,
            access_token: None,
            portal_url,
            gateway_url,
        });
    }
    let token = read_cloud_access_token();
    Json(CloudSessionResponse {
        linked: token.is_some(),
        access_token: token,
        portal_url,
        gateway_url,
    })
}

#[derive(Serialize)]
pub struct CloudGatewayTestResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gateway: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Ping the hosted model-gateway using the linked cloud session bearer token.
pub async fn post_cloud_gateway_test() -> impl IntoResponse {
    let token = match read_cloud_access_token() {
        Some(t) if !t.trim().is_empty() => t,
        _ => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(CloudGatewayTestResponse {
                    ok: false,
                    status: None,
                    gateway: Some(resolve_gateway_host()),
                    snippet: None,
                    error: Some("cloud_session_required".into()),
                }),
            )
                .into_response();
        }
    };

    let gateway = default_gateway_chat_url();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_default();
    let body = serde_json::json!({
        "model": "agnes-chat",
        "messages": [{"role": "user", "content": "ping"}],
        "max_tokens": 8,
    });

    match client
        .post(&gateway)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let ok = resp.status().is_success();
            let text = resp.text().await.unwrap_or_default();
            Json(CloudGatewayTestResponse {
                ok,
                status: Some(status),
                gateway: Some(gateway),
                snippet: Some(text.chars().take(200).collect()),
                error: None,
            })
            .into_response()
        }
        Err(e) => (
            StatusCode::BAD_GATEWAY,
            Json(CloudGatewayTestResponse {
                ok: false,
                status: None,
                gateway: Some(gateway),
                snippet: None,
                error: Some(e.to_string()),
            }),
        )
            .into_response(),
    }
}
