//! Helpers for `config.json` → `mcp.servers` (stdio / HTTP MCP declarations).

use serde_json::{json, Value};

pub const CONFIG_KEY: &str = "mcp";
const SENSITIVE_KEYS: &[&str] = &[
    "api_key",
    "apikey",
    "authorization",
    "bearer",
    "bearer_token",
    "client_secret",
    "oauth_credentials_path",
    "password",
    "refresh_token",
    "secret",
    "token",
];

pub fn read_mcp_servers(cfg: &Value) -> Vec<Value> {
    cfg.get(CONFIG_KEY)
        .and_then(|m| m.get("servers"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

pub fn set_mcp_servers(cfg: &mut Value, servers: Vec<Value>) {
    let root = cfg.as_object_mut().expect("config root must be object");
    let mcp = root
        .entry(CONFIG_KEY)
        .or_insert_with(|| json!({ "browser": { "enabled": false }, "servers": [] }));
    if let Some(obj) = mcp.as_object_mut() {
        obj.insert("servers".into(), Value::Array(servers));
    }
}

pub fn redact_mcp_servers(servers: &[Value]) -> Vec<Value> {
    servers.iter().map(redact_value).collect()
}

fn redact_value(value: &Value) -> Value {
    match value {
        Value::Object(obj) => Value::Object(
            obj.iter()
                .map(|(k, v)| {
                    if is_sensitive_key(k) {
                        (
                            k.clone(),
                            json!({ "configured": !is_empty_secret(v), "preview": "***" }),
                        )
                    } else {
                        (k.clone(), redact_value(v))
                    }
                })
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(redact_value).collect()),
        _ => value.clone(),
    }
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|c| *c != '-' && *c != '_')
        .flat_map(char::to_lowercase)
        .collect::<String>();
    SENSITIVE_KEYS.iter().any(|s| {
        let sensitive = s.replace(['-', '_'], "");
        normalized == sensitive || normalized.ends_with(&sensitive)
    })
}

fn is_empty_secret(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_servers() {
        let mut cfg = json!({ "provider": "z.ai" });
        assert!(read_mcp_servers(&cfg).is_empty());
        set_mcp_servers(&mut cfg, vec![json!({"slug":"fs","command":"echo fs"})]);
        let servers = read_mcp_servers(&cfg);
        assert_eq!(servers.len(), 1);
        assert_eq!(servers[0]["slug"], "fs");
    }

    #[test]
    fn redacts_sensitive_server_fields_recursively() {
        let servers = vec![json!({
            "slug": "remote",
            "bearer_token": "secret-token",
            "headers": {
                "Authorization": "Bearer secret-token",
                "X-Workspace": "safe"
            },
            "nested": [{ "oauth_credentials_path": "/tmp/creds.json" }]
        })];

        let redacted = redact_mcp_servers(&servers);
        assert_eq!(redacted[0]["slug"], "remote");
        assert_eq!(redacted[0]["headers"]["X-Workspace"], "safe");
        assert_eq!(redacted[0]["bearer_token"]["configured"], true);
        assert_eq!(redacted[0]["headers"]["Authorization"]["preview"], "***");
        assert_eq!(
            redacted[0]["nested"][0]["oauth_credentials_path"]["configured"],
            true
        );
    }
}
