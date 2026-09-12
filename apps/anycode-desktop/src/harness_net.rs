//! Outbound Desktop pairing. The product SSO client secret never enters this process.
//! The WebView cannot choose the destination; only the host-configured origin is used.

pub use anycode_harness_cloud818::desktop_pairing::{pairing_target, post_pairing};

#[tauri::command]
pub fn harness_pairing_origin_configured() -> serde_json::Value {
    match std::env::var("ANYCODE_HARNESS_PAIRING_ORIGIN") {
        Ok(origin) if pairing_target(&origin, "/api/harness/pairing/challenges").is_ok() => {
            serde_json::json!({ "configured": true })
        }
        Ok(_) => serde_json::json!({ "configured": false, "error": "invalid origin" }),
        Err(_) => serde_json::json!({ "configured": false }),
    }
}

/// Desktop → server pairing BFF. Origin comes from host config, not the WebView.
#[tauri::command]
pub async fn harness_pairing_post(
    path: String,
    bearer: Option<String>,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let origin = std::env::var("ANYCODE_HARNESS_PAIRING_ORIGIN")
        .map_err(|_| "ANYCODE_HARNESS_PAIRING_ORIGIN is not set on Desktop".to_string())?;
    let has_bearer = bearer
        .as_deref()
        .map(str::trim)
        .is_some_and(|token| !token.is_empty());
    let value = match post_pairing(&origin, &path, bearer.as_deref(), body).await {
        Ok(v) => {
            eprintln!("anycode-desktop: pairing_post path={path} bearer={has_bearer} ok");
            v
        }
        Err(e) => {
            eprintln!("anycode-desktop: pairing_post path={path} bearer={has_bearer} err={e}");
            return Err(e);
        }
    };
    if path.ends_with("/sso/begin") {
        if let Ok(file) = std::env::var("ANYCODE_HARNESS_EXTERNAL_URL_FILE") {
            if !file.is_empty() {
                if let Some(url) = value.get("authorize_url").and_then(|v| v.as_str()) {
                    std::fs::write(&file, url).map_err(|e| e.to_string())?;
                }
            }
        }
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_configured_does_not_echo_host_origin() {
        let v = harness_pairing_origin_configured();
        assert!(v.get("origin").is_none());
        assert!(v["configured"].is_boolean());
        if std::env::var("ANYCODE_HARNESS_PAIRING_ORIGIN").is_err() {
            assert_eq!(v["configured"], false);
        }
    }
}
