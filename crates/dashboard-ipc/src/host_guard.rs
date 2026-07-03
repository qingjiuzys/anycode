//! Host binding checks for dashboard IPC respond paths.

#[must_use]
pub fn is_loopback_host(host: &str) -> bool {
    host == "127.0.0.1" || host == "localhost" || host == "::1"
}
