//! Navigation URL policy (SSRF-aligned with WebFetch).

use crate::error::{BrowserError, BrowserResult};
use url::Url;

fn parse_domain_as_ip_literal(name: &str) -> Option<std::net::IpAddr> {
    let name = name.trim();
    if let Ok(ip) = name.parse::<std::net::IpAddr>() {
        return Some(ip);
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

fn is_blocked_ip(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.octets()[0] == 169 && v4.octets()[1] == 254
        }
        std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unspecified(),
    }
}

pub fn validate_navigation_url(raw: &str) -> BrowserResult<Url> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(BrowserError::InvalidUrl("empty URL".into()));
    }
    let url = Url::parse(raw).map_err(|e| BrowserError::InvalidUrl(e.to_string()))?;
    let scheme = url.scheme().to_ascii_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err(BrowserError::NavigationBlocked(format!(
            "scheme `{scheme}` not allowed (http/https only)"
        )));
    }
    if let Some(host) = url.host() {
        match host {
            url::Host::Domain(name) => {
                let lower = name.to_ascii_lowercase();
                if lower == "localhost" || lower.ends_with(".localhost") {
                    return Err(BrowserError::NavigationBlocked(
                        "localhost not allowed".into(),
                    ));
                }
                if lower == "metadata.google.internal" {
                    return Err(BrowserError::NavigationBlocked(
                        "metadata host not allowed".into(),
                    ));
                }
                if let Some(ip) = parse_domain_as_ip_literal(name) {
                    if is_blocked_ip(ip) {
                        return Err(BrowserError::NavigationBlocked(
                            "private or link-local IP not allowed".into(),
                        ));
                    }
                }
            }
            url::Host::Ipv4(ip) => {
                if is_blocked_ip(std::net::IpAddr::V4(ip)) {
                    return Err(BrowserError::NavigationBlocked(
                        "private or link-local IP not allowed".into(),
                    ));
                }
            }
            url::Host::Ipv6(ip) => {
                if is_blocked_ip(std::net::IpAddr::V6(ip)) {
                    return Err(BrowserError::NavigationBlocked(
                        "private or link-local IP not allowed".into(),
                    ));
                }
            }
        }
    }
    Ok(url)
}

/// Whitelisted CDP methods for `BrowserCdp`.
pub fn cdp_method_allowed(method: &str) -> bool {
    matches!(
        method,
        "Runtime.evaluate"
            | "DOM.getDocument"
            | "DOM.querySelector"
            | "CSS.getComputedStyleForNode"
            | "Accessibility.getFullAXTree"
            | "Page.getLayoutMetrics"
            | "Page.captureScreenshot"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_file_scheme() {
        assert!(validate_navigation_url("file:///etc/passwd").is_err());
    }

    #[test]
    fn allows_https_public() {
        assert!(validate_navigation_url("https://example.com").is_ok());
    }

    #[test]
    fn blocks_localhost() {
        assert!(validate_navigation_url("http://localhost:8080").is_err());
    }
}
