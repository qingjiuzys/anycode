//! Parse MCP servers from env: stdio commands and Streamable HTTP (SSE).

use crate::i18n::{tr, tr_args};
use fluent_bundle::FluentArgs;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

fn expand_tilde_path(raw: &str) -> PathBuf {
    let raw = raw.trim();
    if let Some(rest) = raw.strip_prefix("~/") {
        return dirs::home_dir()
            .map(|h| h.join(rest))
            .unwrap_or_else(|| PathBuf::from(raw));
    }
    PathBuf::from(raw)
}

/// One MCP connection spec (order = connect order).
#[cfg_attr(not(feature = "tools-mcp"), allow(dead_code))]
#[derive(Debug, Clone)]
pub(crate) enum McpServerEntry {
    Stdio {
        slug: String,
        command: String,
    },
    Http {
        slug: String,
        url: String,
        bearer_token: Option<String>,
        /// rmcp credential JSON (`oauth-login --credentials-store`); overrides static `bearer_token`.
        oauth_credentials_path: Option<PathBuf>,
        headers: HashMap<String, String>,
    },
}

/// Return MCP entries; stdio and remote HTTP may be mixed.
#[cfg_attr(not(feature = "tools-mcp"), allow(dead_code))]
pub fn mcp_server_entries_from_env() -> Vec<McpServerEntry> {
    if let Ok(raw) = std::env::var("ANYCODE_MCP_SERVERS") {
        let t = raw.trim();
        if !t.is_empty() {
            let parsed = parse_mcp_servers_json(t);
            if !parsed.is_empty() {
                return parsed;
            }
        }
    }
    if let Ok(cmd) = std::env::var("ANYCODE_MCP_COMMAND") {
        let c = cmd.trim();
        if !c.is_empty() {
            let slug =
                std::env::var("ANYCODE_MCP_SERVER").unwrap_or_else(|_| "default".to_string());
            return vec![McpServerEntry::Stdio {
                slug,
                command: c.to_string(),
            }];
        }
    }
    vec![]
}

fn parse_mcp_servers_json(raw: &str) -> Vec<McpServerEntry> {
    let v: Value = match serde_json::from_str(raw) {
        Ok(x) => x,
        Err(e) => {
            let mut a = FluentArgs::new();
            a.set("err", e.to_string());
            tracing::warn!(
                target: "anycode_cli",
                "{}",
                tr_args("log-mcp-json-skip", &a)
            );
            return vec![];
        }
    };
    let Some(arr) = v.as_array() else {
        tracing::warn!(target: "anycode_cli", "{}", tr("log-mcp-json-array"));
        return vec![];
    };
    let mut out = Vec::new();
    for (i, item) in arr.iter().cloned().enumerate() {
        if let Some(s) = item.as_str() {
            let s = s.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with("http://") || s.starts_with("https://") {
                let slug = format!("mcp{}", i);
                out.push(http_entry_from_object(
                    i,
                    &serde_json::json!({
                        "slug": slug,
                        "url": s
                    }),
                ));
            } else {
                out.push(McpServerEntry::Stdio {
                    slug: format!("mcp{}", i),
                    command: s.to_string(),
                });
            }
            continue;
        }
        let Some(obj) = item.as_object() else {
            continue;
        };
        let ty = obj
            .get("type")
            .and_then(|x| x.as_str())
            .map(|s| s.trim().to_ascii_lowercase())
            .unwrap_or_default();
        let url = obj
            .get("url")
            .and_then(|x| x.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let cmd = obj
            .get("command")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim();
        let is_http_ty = matches!(
            ty.as_str(),
            "http" | "sse" | "streamable" | "streamablehttp"
        );
        let looks_like_url =
            url.is_some_and(|u| u.starts_with("http://") || u.starts_with("https://"));
        let use_http = looks_like_url && (is_http_ty || cmd.is_empty());
        if use_http {
            out.push(http_entry_from_object(i, &item));
            continue;
        }
        if cmd.is_empty() {
            let mut a = FluentArgs::new();
            a.set("i", i as i64);
            tracing::warn!(target: "anycode_cli", "{}", tr_args("log-mcp-entry-skip", &a));
            continue;
        }
        let slug = obj
            .get("slug")
            .or_else(|| obj.get("name"))
            .and_then(|x| x.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("mcp{}", i));
        out.push(McpServerEntry::Stdio {
            slug,
            command: cmd.to_string(),
        });
    }
    out
}

fn http_entry_from_object(i: usize, item: &Value) -> McpServerEntry {
    let obj = item.as_object().cloned().unwrap_or_default();
    let url = obj
        .get("url")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let slug = obj
        .get("slug")
        .or_else(|| obj.get("name"))
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("mcp{}", i));
    let mut bearer_token = obj
        .get("bearer_token")
        .or_else(|| obj.get("oauth_token"))
        .or_else(|| obj.get("access_token"))
        .and_then(|x| x.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let env_key = format!(
        "ANYCODE_MCP_BEARER_{}",
        slug.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    );
    if let Ok(t) = std::env::var(&env_key) {
        let t = t.trim();
        if !t.is_empty() {
            bearer_token = Some(t.to_string());
        }
    }
    let oauth_env_key = format!(
        "ANYCODE_MCP_OAUTH_{}",
        slug.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
    );
    let mut oauth_credentials_path = obj
        .get("oauth_credentials_path")
        .or_else(|| obj.get("oauth_store"))
        .or_else(|| obj.get("credentials_store"))
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| expand_tilde_path(s));
    if let Ok(p) = std::env::var(&oauth_env_key) {
        let p = p.trim();
        if !p.is_empty() {
            oauth_credentials_path = Some(expand_tilde_path(p));
        }
    }
    let mut headers = HashMap::new();
    if let Some(h) = obj.get("headers").and_then(|x| x.as_object()) {
        for (k, v) in h {
            if let Some(vs) = v.as_str() {
                headers.insert(k.clone(), vs.to_string());
            }
        }
    }
    McpServerEntry::Http {
        slug,
        url,
        bearer_token,
        oauth_credentials_path,
        headers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_objects_and_strings() {
        let j = r#"[
            {"slug":"fs","command":"echo fs"},
            "echo bare"
        ]"#;
        let v = parse_mcp_servers_json(j);
        assert_eq!(v.len(), 2);
        match &v[0] {
            McpServerEntry::Stdio { slug, command } => {
                assert_eq!(slug, "fs");
                assert_eq!(command, "echo fs");
            }
            _ => panic!("expected stdio"),
        }
        match &v[1] {
            McpServerEntry::Stdio { slug, command } => {
                assert_eq!(slug, "mcp1");
                assert_eq!(command, "echo bare");
            }
            _ => panic!("expected stdio"),
        }
    }

    #[test]
    fn parse_http_entry() {
        let j = r#"[
            {"slug":"api","type":"http","url":"https://example.com/mcp","bearer_token":"t"}
        ]"#;
        let v = parse_mcp_servers_json(j);
        match &v[0] {
            McpServerEntry::Http {
                slug,
                url,
                bearer_token,
                oauth_credentials_path,
                ..
            } => {
                assert_eq!(slug, "api");
                assert_eq!(url, "https://example.com/mcp");
                assert_eq!(bearer_token.as_deref(), Some("t"));
                assert!(oauth_credentials_path.is_none());
            }
            _ => panic!("expected http"),
        }
    }

    #[test]
    fn parse_http_oauth_store_path() {
        let j = r#"[
            {"slug":"x","type":"http","url":"https://ex/mcp","oauth_credentials_path":"/tmp/mcp-oauth.json"}
        ]"#;
        let v = parse_mcp_servers_json(j);
        match &v[0] {
            McpServerEntry::Http {
                oauth_credentials_path,
                ..
            } => {
                assert_eq!(
                    oauth_credentials_path.as_ref().map(|p| p.as_path()),
                    Some(std::path::Path::new("/tmp/mcp-oauth.json"))
                );
            }
            _ => panic!("expected http"),
        }
    }
}
