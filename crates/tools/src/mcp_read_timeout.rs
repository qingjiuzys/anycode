//! Configurable JSON-RPC line read timeout for MCP stdio transports.

use std::time::Duration;

/// Environment variable: override per-line read timeout (seconds) for MCP stdio JSON-RPC.
pub const ANYCODE_MCP_READ_TIMEOUT_SECS: &str = "ANYCODE_MCP_READ_TIMEOUT_SECS";

const MAX_SECS: u64 = 86_400;

fn parse_timeout_secs(raw: &str) -> Option<Duration> {
    let n = raw.trim().parse::<u64>().ok()?;
    if n == 0 || n > MAX_SECS {
        return None;
    }
    Some(Duration::from_secs(n))
}

/// When `ANYCODE_MCP_READ_TIMEOUT_SECS` is unset or invalid, uses `default`.
#[must_use]
pub fn mcp_jsonrpc_line_timeout(default: Duration) -> Duration {
    std::env::var(ANYCODE_MCP_READ_TIMEOUT_SECS)
        .ok()
        .and_then(|s| parse_timeout_secs(&s))
        .unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_timeout_accepts_trimmed() {
        assert_eq!(parse_timeout_secs("30"), Some(Duration::from_secs(30)));
        assert_eq!(parse_timeout_secs("  90 \n"), Some(Duration::from_secs(90)));
    }

    #[test]
    fn parse_timeout_rejects_bad() {
        assert_eq!(parse_timeout_secs("0"), None);
        assert_eq!(parse_timeout_secs("86401"), None);
        assert_eq!(parse_timeout_secs("x"), None);
    }
}
