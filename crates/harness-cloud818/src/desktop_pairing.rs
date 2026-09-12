//! Secret-free Desktop → pairing BFF HTTP client.
//! PRODUCT_SSO_CLIENT_SECRET never enters this module. The WebView cannot
//! choose the destination; the host supplies the origin.

const PAIRING_PATHS: &[&str] = &[
    "/api/harness/pairing/challenges",
    "/api/harness/pairing/confirm",
    "/api/harness/pairing/revoke",
    "/api/harness/pairing/sso/begin",
    "/api/harness/pairing/sso/complete",
    "/api/harness/pairing/sso/poll",
];

pub fn pairing_target(origin: &str, path: &str) -> Result<reqwest::Url, String> {
    let origin = origin.trim().trim_end_matches('/');
    if origin.is_empty() {
        return Err("pairing origin required".into());
    }
    let url = reqwest::Url::parse(origin).map_err(|_| "pairing origin is not a URL".to_string())?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err("pairing origin must not include credentials".into());
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("pairing origin must not include query or fragment".into());
    }
    if url.path() != "/" && !url.path().is_empty() {
        return Err("pairing origin must not include a path".into());
    }
    let https = url.scheme() == "https";
    let loopback_http = url.scheme() == "http"
        && matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"));
    if !https && !loopback_http {
        return Err("pairing origin must be https or loopback http".into());
    }
    if !PAIRING_PATHS.contains(&path) {
        return Err("pairing path is not allowlisted".into());
    }
    url.join(path)
        .map_err(|_| "pairing URL join failed".to_string())
}

pub async fn post_pairing(
    origin: &str,
    path: &str,
    bearer: Option<&str>,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = pairing_target(origin, path)?;
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let mut req = client
        .post(url)
        .header("content-type", "application/json")
        .json(&body);
    if let Some(token) = bearer.map(str::trim).filter(|t| !t.is_empty()) {
        req = req.header("authorization", format!("Bearer {token}"));
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    let status = res.status();
    let json = res
        .json::<serde_json::Value>()
        .await
        .unwrap_or_else(|_| serde_json::json!({}));
    if !status.is_success() {
        let message = json
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("pairing request failed");
        return Err(format!("{status}: {message}"));
    }
    Ok(json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_target_allows_loopback_and_https_only() {
        assert!(
            pairing_target("http://127.0.0.1:18781", "/api/harness/pairing/challenges").is_ok()
        );
        assert!(pairing_target("http://127.0.0.1:18781", "/api/harness/pairing/sso/begin").is_ok());
        assert!(pairing_target("https://anycode.work", "/api/harness/pairing/confirm").is_ok());
        assert!(pairing_target("http://example.com", "/api/harness/pairing/challenges").is_err());
        assert!(pairing_target("http://127.0.0.1:9", "/api/harness/graphs/start").is_err());
        assert!(pairing_target(
            "http://user:pass@127.0.0.1:9",
            "/api/harness/pairing/confirm"
        )
        .is_err());
        assert!(
            pairing_target("http://127.0.0.1:9/extra", "/api/harness/pairing/confirm").is_err()
        );
    }

    #[tokio::test]
    async fn post_pairing_sends_bearer_to_loopback_bff() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let origin = format!("http://127.0.0.1:{port}");
        std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 1024];
            while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
                let n = stream.read(&mut chunk).unwrap_or(0);
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
            }
            let req = String::from_utf8_lossy(&buf).to_ascii_lowercase();
            assert!(req.contains("post /api/harness/pairing/challenges"));
            assert!(req.contains("authorization: bearer test-sso"));
            let body = br#"{"device_id":"11111111-1111-1111-1111-111111111111","challenge":"c","expires_in":300}"#;
            let _ = write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                std::str::from_utf8(body).unwrap()
            );
        });
        let json = post_pairing(
            &origin,
            "/api/harness/pairing/challenges",
            Some("test-sso"),
            serde_json::json!({"label":"desktop"}),
        )
        .await
        .expect("loopback pairing post");
        assert_eq!(json["expires_in"], 300);
    }
}
