//! Configurable JSON-RPC line read timeout for MCP stdio transports.

use anycode_core::CoreError;
use std::time::Duration;

/// Environment variable: override per-line read timeout (seconds) for MCP stdio JSON-RPC.
pub const ANYCODE_MCP_READ_TIMEOUT_SECS: &str = "ANYCODE_MCP_READ_TIMEOUT_SECS";

/// Optional wall-clock cap (seconds) for a single MCP **`tools/call`** round-trip (stdio, rmcp, legacy SSE, and `ANYCODE_MCP_COMMAND` one-shot).
pub const ANYCODE_MCP_CALL_TIMEOUT_SECS: &str = "ANYCODE_MCP_CALL_TIMEOUT_SECS";

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

/// When unset or invalid, no wall-clock cap is applied (only per-line read timeouts apply).
#[must_use]
pub fn mcp_tools_call_wall_timeout() -> Option<Duration> {
    std::env::var(ANYCODE_MCP_CALL_TIMEOUT_SECS)
        .ok()
        .and_then(|s| parse_timeout_secs(&s))
}

#[must_use]
pub fn mcp_wall_timeout_core_error(dur: Duration, server_hint: &str) -> CoreError {
    CoreError::IoError(std::io::Error::new(
        std::io::ErrorKind::TimedOut,
        format!(
            "MCP tools/call wall-clock timeout after {}s (set {} to adjust; server/hint={})",
            dur.as_secs(),
            ANYCODE_MCP_CALL_TIMEOUT_SECS,
            server_hint
        ),
    ))
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

    #[test]
    fn call_wall_timeout_env_parses_like_read_timeout() {
        assert_eq!(
            parse_timeout_secs("45"),
            Some(std::time::Duration::from_secs(45))
        );
    }

    #[test]
    fn wall_timeout_core_error_is_timed_out_io() {
        let dur = Duration::from_secs(2);
        let e = mcp_wall_timeout_core_error(dur, "test-server");
        match e {
            CoreError::IoError(io) => {
                assert_eq!(io.kind(), std::io::ErrorKind::TimedOut);
                let msg = io.to_string();
                assert!(msg.contains("test-server"));
                assert!(msg.contains(ANYCODE_MCP_CALL_TIMEOUT_SECS));
            }
            _ => panic!("expected IoError"),
        }
    }
}
