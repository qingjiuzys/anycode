//! LAN handoff security helpers.

use std::net::IpAddr;

/// True for RFC1918, link-local, and loopback (dev).
#[must_use]
pub fn is_private_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_private()
                || v4.is_loopback()
                || v4.is_link_local()
                || v4.octets()[0] == 169 && v4.octets()[1] == 254
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

pub fn peer_addr_from_headers(
    forwarded: Option<&str>,
    real_ip: Option<&str>,
    remote: Option<std::net::SocketAddr>,
) -> Option<IpAddr> {
    if let Some(fwd) = forwarded {
        if let Some(first) = fwd.split(',').next() {
            if let Ok(ip) = first.trim().parse() {
                return Some(ip);
            }
        }
    }
    if let Some(rip) = real_ip {
        if let Ok(ip) = rip.trim().parse() {
            return Some(ip);
        }
    }
    remote.map(|a| a.ip())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn private_ipv4_detected() {
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 1, 10))));
        assert!(is_private_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(!is_private_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }
}
