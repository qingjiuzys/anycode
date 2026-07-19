use axum::Router;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};

/// SPA static assets with fallback to index.html for client-side routes.
///
/// Use [`ServeDir::fallback`] (not `not_found_service`) so `/login` and other
/// client routes return **200** with `index.html`. `not_found_service` keeps
/// the 404 status, which breaks some browsers/clients that refuse to run JS
/// on error documents.
pub fn portal_router(portal_dir: PathBuf) -> Router {
    let index = portal_dir.join("index.html");
    let serve_dir = ServeDir::new(portal_dir).fallback(ServeFile::new(index));
    Router::new().fallback_service(serve_dir)
}
