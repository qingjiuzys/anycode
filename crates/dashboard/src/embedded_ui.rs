//! Compile-time embedded dashboard UI (`dashboard-ui/dist`).

#[cfg(feature = "embedded-ui")]
mod inner {
    use axum::{
        body::Body,
        http::{header, StatusCode, Uri},
        response::{IntoResponse, Response},
        Json,
    };
    use rust_embed::Embed;
    use serde_json::json;

    #[derive(Embed)]
    #[folder = "../dashboard-ui/dist/"]
    struct UiAssets;

    fn mime(path: &str) -> &'static str {
        if path.ends_with(".html") {
            "text/html; charset=utf-8"
        } else if path.ends_with(".js") {
            "application/javascript; charset=utf-8"
        } else if path.ends_with(".css") {
            "text/css; charset=utf-8"
        } else if path.ends_with(".svg") {
            "image/svg+xml"
        } else if path.ends_with(".png") {
            "image/png"
        } else if path.ends_with(".woff2") {
            "font/woff2"
        } else if path.ends_with(".json") {
            "application/json; charset=utf-8"
        } else {
            "application/octet-stream"
        }
    }

    fn serve_path(path: &str) -> Option<Response> {
        let key = path.trim_start_matches('/');
        let key = if key.is_empty() { "index.html" } else { key };
        let (asset, mime_key) = if let Some(file) = UiAssets::get(key) {
            (file, key)
        } else {
            let file = UiAssets::get("index.html")?;
            (file, "index.html")
        };
        let content_type = mime(mime_key);
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type)
            .body(Body::from(asset.data.into_owned()))
            .ok()
    }

    pub async fn fallback(uri: Uri) -> impl IntoResponse {
        if uri.path().starts_with("/api/") {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "API route not found" })),
            )
                .into_response();
        }
        let path = uri.path();
        // Artifacts page route `/assets` collides with Vite's `/assets/*` bundle prefix.
        if path == "/assets" {
            if let Some(resp) = serve_path("index.html") {
                return resp.into_response();
            }
        }
        if let Some(rest) = path.strip_prefix("/assets/") {
            if !rest.is_empty() {
                if let Some(resp) = serve_path(&format!("assets/{rest}")) {
                    return resp.into_response();
                }
            }
        }
        serve_path(path).unwrap_or_else(|| {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap()
        })
    }

    #[must_use]
    pub fn available() -> bool {
        UiAssets::get("index.html").is_some()
    }
}

#[cfg(feature = "embedded-ui")]
pub use inner::{available, fallback};

#[cfg(not(feature = "embedded-ui"))]
pub async fn fallback(_uri: axum::http::Uri) -> axum::response::Response {
    axum::response::Response::builder()
        .status(axum::http::StatusCode::NOT_FOUND)
        .body(axum::body::Body::empty())
        .unwrap()
}

#[cfg(not(feature = "embedded-ui"))]
#[must_use]
pub fn available() -> bool {
    false
}
