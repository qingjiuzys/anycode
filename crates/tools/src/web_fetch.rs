//! `WebFetch` — 拉取 URL 正文（大小限制）；`prompt` 字段仅作元数据提示，不在工具内二次调用模型。

use crate::services::ToolServices;
use anycode_core::prelude::*;
use async_trait::async_trait;
use reqwest::redirect::Policy;
use serde::Deserialize;
use std::sync::Arc;
use std::time::Instant;

/// Max HTTP redirects to follow (OpenClaw fetch-guard uses a small cap).
const MAX_FETCH_REDIRECTS: usize = 5;

pub struct WebFetchTool {
    security_policy: SecurityPolicy,
    services: Arc<ToolServices>,
}

impl WebFetchTool {
    pub fn new(services: Arc<ToolServices>) -> Self {
        Self {
            security_policy: SecurityPolicy::sensitive_mutation(),
            services,
        }
    }
}

#[derive(Deserialize)]
struct WfInput {
    url: String,
    #[serde(default)]
    prompt: String,
}

/// Decimal IPv4 hostnames (e.g. `2130706433` → 127.0.0.1) bypass dotted-quad checks unless handled here.
fn parse_domain_as_ip_literal(name: &str) -> Option<std::net::IpAddr> {
    let name = name.trim();
    if let Ok(ip) = name.parse::<std::net::IpAddr>() {
        return Some(ip);
    }
    if name.len() > 2 {
        let hex = name.strip_prefix("0x").or_else(|| name.strip_prefix("0X"));
        if let Some(digits) = hex {
            if !digits.is_empty() && digits.chars().all(|c| c.is_ascii_hexdigit()) {
                if let Ok(n) = u32::from_str_radix(digits, 16) {
                    return Some(std::net::IpAddr::V4(std::net::Ipv4Addr::from(
                        n.to_be_bytes(),
                    )));
                }
            }
        }
    }
    if !name.is_empty() && name.chars().all(|c| c.is_ascii_digit()) {
        if let Ok(n) = name.parse::<u32>() {
            return Some(std::net::IpAddr::V4(std::net::Ipv4Addr::from(
                n.to_be_bytes(),
            )));
        }
    }
    None
}

/// Reject loopback, RFC1918, link-local, and metadata hosts (OpenClaw fetch-guard subset).
fn fetch_url_host_blocked(url: &url::Url) -> Option<&'static str> {
    if let Some(host) = url.host() {
        if let url::Host::Domain(name) = host {
            let lower = name.to_ascii_lowercase();
            if lower == "localhost" || lower.ends_with(".localhost") {
                return Some("localhost host not allowed");
            }
            if lower == "metadata.google.internal" {
                return Some("metadata host not allowed");
            }
            if let Some(ip) = parse_domain_as_ip_literal(name) {
                if is_blocked_fetch_ip(ip) {
                    return Some("private or link-local IP not allowed");
                }
            }
        } else if let url::Host::Ipv4(ip) = host {
            if is_blocked_fetch_ip(std::net::IpAddr::V4(ip)) {
                return Some("private or link-local IP not allowed");
            }
        } else if let url::Host::Ipv6(ip) = host {
            if is_blocked_fetch_ip(std::net::IpAddr::V6(ip)) {
                return Some("private or link-local IP not allowed");
            }
        }
    }
    None
}

/// Strip userinfo from URL before fetch and before persisting redirect targets.
fn sanitize_fetch_url(mut url: url::Url) -> url::Url {
    let _ = url.set_username("");
    let _ = url.set_password(None);
    url
}

fn resolve_redirect_location(base: &url::Url, location: &str) -> Option<url::Url> {
    url::Url::parse(location)
        .ok()
        .or_else(|| base.join(location).ok())
        .map(sanitize_fetch_url)
}

/// DNS rebinding guard: resolve hostname and reject if any A/AAAA is private/link-local.
async fn fetch_url_dns_blocked(url: &url::Url) -> Option<&'static str> {
    let url::Host::Domain(name) = url.host()? else {
        return None;
    };
    let port = url.port_or_known_default().unwrap_or(443);
    let addrs = match tokio::net::lookup_host((name, port)).await {
        Ok(a) => a,
        Err(_) => return Some("DNS resolution failed"),
    };
    let mut any = false;
    for addr in addrs {
        any = true;
        if is_blocked_fetch_ip(addr.ip()) {
            return Some("resolved to private or link-local IP not allowed");
        }
    }
    if !any {
        return Some("DNS resolution failed");
    }
    None
}

fn is_blocked_fetch_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_documentation()
                || v4.octets() == [0, 0, 0, 0]
        }
        std::net::IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_blocked_fetch_ip(std::net::IpAddr::V4(v4));
            }
            v6.is_loopback() || v6.is_unique_local() || (v6.segments()[0] & 0xffc0) == 0xfe80
        }
    }
}

fn strip_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn description(&self) -> &str {
        "Fetch content from a URL (text/html), with byte limit. Host processes `prompt` separately."
    }

    fn api_tool_description(&self) -> String {
        format!(
            "{}\n\n\
            HTTP(S) fetch for page text; HTML is lightly stripped.\n\
            - Subject to size limits; very large responses are truncated or rejected.\n\
            - `prompt` is **not** sent to a model inside the tool—it is metadata for the host/session only.\n\
            - May require approval depending on security policy.",
            self.description()
        )
    }

    fn schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "HTTP(S) URL to fetch" },
                "prompt": { "type": "string", "description": "Host-side prompt hint (not executed inside this tool)" }
            },
            "required": ["url"]
        })
    }

    fn permission_mode(&self) -> PermissionMode {
        PermissionMode::Default
    }

    fn security_policy(&self) -> Option<&SecurityPolicy> {
        Some(&self.security_policy)
    }

    async fn execute(&self, input: ToolInput) -> Result<ToolOutput, CoreError> {
        let start = Instant::now();
        let wf: WfInput =
            serde_json::from_value(input.input).map_err(CoreError::SerializationError)?;

        let url = url::Url::parse(&wf.url)
            .map_err(|e| CoreError::Other(anyhow::anyhow!("bad url: {}", e)))?;
        let url = sanitize_fetch_url(url);
        if !matches!(url.scheme(), "http" | "https") {
            return Ok(ToolOutput {
                result: serde_json::json!({"error": "only http/https allowed"}),
                error: Some("scheme".into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        if let Some(reason) = fetch_url_host_blocked(&url) {
            return Ok(ToolOutput {
                result: serde_json::json!({"error": reason}),
                error: Some(reason.into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }
        if let Some(reason) = fetch_url_dns_blocked(&url).await {
            return Ok(ToolOutput {
                result: serde_json::json!({"error": reason}),
                error: Some(reason.into()),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let client = reqwest::Client::builder()
            .user_agent("anycode-tools/0.1")
            .redirect(Policy::none())
            .build()
            .map_err(|e| CoreError::Other(anyhow::anyhow!("http client: {}", e)))?;
        let mut current = url.clone();
        let mut redirects_followed = 0usize;
        let resp = loop {
            if let Some(reason) = fetch_url_host_blocked(&current) {
                return Ok(ToolOutput {
                    result: serde_json::json!({"error": reason}),
                    error: Some(reason.into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            if let Some(reason) = fetch_url_dns_blocked(&current).await {
                return Ok(ToolOutput {
                    result: serde_json::json!({"error": reason}),
                    error: Some(reason.into()),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
            let resp = client
                .get(current.clone())
                .send()
                .await
                .map_err(|e| CoreError::Other(anyhow::anyhow!("fetch failed: {}", e)))?;
            if resp.status().is_redirection() {
                if redirects_followed >= MAX_FETCH_REDIRECTS {
                    return Ok(ToolOutput {
                        result: serde_json::json!({
                            "error": format!("too many redirects (max {})", MAX_FETCH_REDIRECTS)
                        }),
                        error: Some("redirect_limit".into()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
                let Some(loc) = resp
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                else {
                    return Ok(ToolOutput {
                        result: serde_json::json!({"error": "redirect missing Location header"}),
                        error: Some("redirect".into()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                };
                let Some(next) = resolve_redirect_location(&current, loc) else {
                    return Ok(ToolOutput {
                        result: serde_json::json!({"error": "invalid redirect Location"}),
                        error: Some("redirect".into()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                };
                if !matches!(next.scheme(), "http" | "https") {
                    return Ok(ToolOutput {
                        result: serde_json::json!({"error": "only http/https allowed"}),
                        error: Some("scheme".into()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    });
                }
                redirects_followed += 1;
                current = next;
                continue;
            }
            break resp;
        };

        let code = resp.status().as_u16();
        let code_text = resp.status().canonical_reason().unwrap_or("").to_string();
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| CoreError::Other(anyhow::anyhow!("read body: {}", e)))?;

        let max = self.services.max_fetch_bytes as usize;
        let truncated = bytes.len() > max;
        let slice = if truncated { &bytes[..max] } else { &bytes[..] };

        let raw_str = String::from_utf8_lossy(slice);
        let text = if raw_str.contains('<') && raw_str.contains('>') {
            strip_tags(&raw_str)
        } else {
            raw_str.into_owned()
        };

        let result_text = if text.len() > 256_000 {
            text.chars().take(256_000).collect::<String>() + "\n...<truncated>"
        } else {
            text
        };

        Ok(ToolOutput {
            result: serde_json::json!({
                "bytes": bytes.len(),
                "code": code,
                "codeText": code_text,
                "result": result_text,
                "durationMs": start.elapsed().as_millis() as u64,
                "prompt_note": wf.prompt,
                "body_truncated_to_max_fetch": truncated
            }),
            error: if (200..400).contains(&code) {
                None
            } else {
                Some(format!("HTTP {}", code))
            },
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_loopback_and_private_hosts() {
        for u in [
            "http://127.0.0.1/",
            "http://[::1]/",
            "http://localhost/",
            "http://192.168.1.1/",
            "http://10.0.0.1/",
            "http://169.254.169.254/",
            "http://0.0.0.0/",
            "http://metadata.google.internal/",
        ] {
            let url = url::Url::parse(u).unwrap();
            assert!(
                fetch_url_host_blocked(&url).is_some(),
                "expected block for {u}"
            );
        }
    }

    #[test]
    fn allows_public_hostnames() {
        let url = url::Url::parse("https://example.com/page").unwrap();
        assert!(fetch_url_host_blocked(&url).is_none());
    }

    #[test]
    fn strips_credentials_from_url() {
        let url = url::Url::parse("https://user:secret@example.com/path").unwrap();
        let clean = sanitize_fetch_url(url);
        assert!(clean.username().is_empty());
        assert!(clean.password().is_none());
        assert_eq!(clean.host_str(), Some("example.com"));
    }

    #[test]
    fn resolves_relative_redirect_against_base() {
        let base = url::Url::parse("https://example.com/a/b").unwrap();
        let next = resolve_redirect_location(&base, "/c").unwrap();
        assert_eq!(next.as_str(), "https://example.com/c");
    }

    #[test]
    fn blocks_redirect_target_private_host() {
        let base = url::Url::parse("https://example.com/").unwrap();
        let next = resolve_redirect_location(&base, "http://127.0.0.1/").unwrap();
        assert!(fetch_url_host_blocked(&next).is_some());
    }

    #[test]
    fn blocks_decimal_ipv4_hostname() {
        let url = url::Url::parse("http://2130706433/").unwrap();
        assert!(fetch_url_host_blocked(&url).is_some());
    }

    #[test]
    fn blocks_hex_ipv4_hostname() {
        let url = url::Url::parse("http://0x7f000001/").unwrap();
        assert!(fetch_url_host_blocked(&url).is_some());
    }

    #[test]
    fn blocks_ipv4_mapped_loopback_in_ipv6_literal() {
        let url = url::Url::parse("http://[::ffff:127.0.0.1]/").unwrap();
        assert!(fetch_url_host_blocked(&url).is_some());
    }

    #[test]
    fn redirect_location_strips_embedded_credentials() {
        let base = url::Url::parse("https://example.com/").unwrap();
        let next =
            resolve_redirect_location(&base, "https://user:secret@example.com/next").unwrap();
        assert!(next.username().is_empty());
        assert!(next.password().is_none());
    }
}
