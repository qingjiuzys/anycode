//! Locate built `dashboard-ui/dist` for single-binary serving.

use std::path::PathBuf;

/// Env override: `ANYCODE_DASHBOARD_STATIC` → directory containing `index.html`.
#[must_use]
pub fn discover_ui_dist() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("ANYCODE_DASHBOARD_STATIC") {
        let p = PathBuf::from(raw);
        if p.join("index.html").is_file() {
            return Some(p);
        }
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for rel in ["../dashboard-ui/dist", "../../crates/dashboard-ui/dist"] {
        let p = manifest.join(rel);
        if p.join("index.html").is_file() {
            return Some(p.canonicalize().unwrap_or(p));
        }
    }
    None
}

/// Whether any UI source is available (filesystem dist or compile-time embed).
#[must_use]
pub fn ui_available() -> bool {
    discover_ui_dist().is_some() || crate::embedded_ui::available()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discover_returns_none_when_missing() {
        // In CI the dist folder may not exist; just ensure no panic.
        let _ = discover_ui_dist();
    }
}
