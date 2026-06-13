//! Bundled Playwright browser MCP (desktop resources or `ANYCODE_BROWSER_MCP_ROOT`).

#![cfg_attr(not(feature = "tools-mcp"), allow(dead_code))]

use std::path::{Path, PathBuf};

const BROWSER_SLUG: &str = "browser";

/// Resolve staged browser MCP bundle (run.sh + node_modules + browsers/).
pub(crate) fn resolve_browser_mcp_bundle_root() -> Option<PathBuf> {
    if let Ok(raw) = std::env::var("ANYCODE_BROWSER_MCP_ROOT") {
        let p = PathBuf::from(raw.trim());
        if is_browser_bundle(&p) {
            return Some(p);
        }
    }
    None
}

fn is_browser_bundle(root: &Path) -> bool {
    root.join("run.sh").is_file() && root.join("node_modules/@playwright/mcp/cli.js").is_file()
}

/// Shell command for [`McpStdioSession::connect`] (`sh -c`).
pub(crate) fn browser_mcp_stdio_command(root: &Path) -> String {
    let run = root.join("run.sh");
    shell_escape(run.to_string_lossy().as_ref())
}

fn shell_escape(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "/._-".contains(c))
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

pub(crate) fn browser_mcp_slug() -> &'static str {
    BROWSER_SLUG
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_escape_simple_path() {
        assert_eq!(shell_escape("/tmp/run.sh"), "/tmp/run.sh");
    }
}
