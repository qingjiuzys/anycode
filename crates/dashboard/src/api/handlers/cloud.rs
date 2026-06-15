use super::*;
use anycode_llm::{cloud_session_path, read_cloud_access_token};
use serde::Serialize;

#[derive(Serialize)]
pub struct CloudSessionResponse {
    pub linked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub portal_url: Option<String>,
}

pub async fn get_cloud_session() -> Json<CloudSessionResponse> {
    let portal_url = std::env::var("ANYCODE_ACCOUNT_PORTAL_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("ANYCODE_ACCOUNT_API_URL")
                .ok()
                .filter(|s| !s.trim().is_empty())
        });
    let path = cloud_session_path();
    if !path.is_file() {
        return Json(CloudSessionResponse {
            linked: false,
            access_token: None,
            portal_url,
        });
    }
    let token = read_cloud_access_token();
    Json(CloudSessionResponse {
        linked: token.is_some(),
        access_token: token,
        portal_url,
    })
}
