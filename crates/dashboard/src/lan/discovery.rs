//! mDNS discovery for `_anycode._tcp.local`.

use crate::lan::peer::LanPeer;
use crate::lan::LanHub;
use anyhow::{Context, Result};
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

pub const MDNS_SERVICE_TYPE: &str = "_anycode._tcp.local.";
pub const DEFAULT_LAN_PORT: u16 = 43181;

pub fn spawn_discovery(hub: Arc<LanHub>) {
    tokio::spawn(async move {
        if let Err(e) = run_discovery(hub).await {
            warn!(error = %e, "LAN mDNS discovery stopped");
        }
    });
}

/// Best-effort primary private IPv4 for LAN handoff URLs.
pub fn primary_lan_ip() -> Option<std::net::IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|a| a.ip())
}

async fn run_discovery(hub: Arc<LanHub>) -> Result<()> {
    let settings = hub.settings_snapshot().await;
    if !settings.discovery_enabled {
        debug!("LAN discovery disabled in settings");
        return Ok(());
    }

    let mdns = ServiceDaemon::new().context("create mDNS daemon")?;
    let lan_port = settings.lan_port;
    let host_ip = primary_lan_ip().unwrap_or(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let host_str = match host_ip {
        IpAddr::V4(v4) => v4.to_string(),
        IpAddr::V6(v6) => v6.to_string(),
    };

    let props: Vec<(String, String)> = vec![
        ("instance_id".into(), hub.instance.instance_id.clone()),
        ("device_name".into(), settings.display_name.clone()),
        ("version".into(), hub.version.clone()),
        ("lan_port".into(), lan_port.to_string()),
    ];

    let service_name = format!("{}._anycode._tcp.local.", hub.instance.instance_id);
    let service_info = ServiceInfo::new(
        MDNS_SERVICE_TYPE,
        &service_name,
        &host_str,
        host_ip,
        lan_port,
        &props[..],
    )
    .context("build mDNS service info")?;

    mdns.register(service_info)
        .context("register mDNS service")?;
    info!(host = %host_str, port = lan_port, "LAN mDNS advertised");

    let receiver = mdns.browse(MDNS_SERVICE_TYPE).context("browse mDNS")?;

    let hub_clone = Arc::clone(&hub);
    tokio::task::spawn_blocking(move || loop {
        match receiver.recv_timeout(Duration::from_secs(5)) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                handle_resolved(&hub_clone, info);
            }
            Ok(ServiceEvent::ServiceRemoved(name, _)) => {
                debug!(service = %name, "mDNS service removed");
            }
            Ok(_) => {}
            Err(_) => {
                hub_clone.remove_stale_peers(120);
            }
        }
    });

    // Keep future alive
    std::future::pending::<()>().await;
    Ok(())
}

fn handle_resolved(hub: &LanHub, info: ServiceInfo) {
    let props = info.get_properties();
    let instance_id = props
        .get("instance_id")
        .map(|v| v.val_str().to_string())
        .unwrap_or_else(|| info.get_fullname().to_string());
    if instance_id == hub.instance.instance_id {
        return;
    }
    let device_name = props
        .get("device_name")
        .map(|v| v.val_str().to_string())
        .unwrap_or_else(|| instance_id.clone());
    let version = props
        .get("version")
        .map(|v| v.val_str().to_string())
        .unwrap_or_else(|| "unknown".into());
    let lan_port = props
        .get("lan_port")
        .and_then(|v| v.val_str().parse().ok())
        .unwrap_or(DEFAULT_LAN_PORT);

    let host = info
        .get_addresses()
        .iter()
        .find(|ip| ip.is_ipv4())
        .or_else(|| info.get_addresses().iter().next())
        .map(|ip| ip.to_string())
        .unwrap_or_else(|| "127.0.0.1".into());

    hub.upsert_peer(LanPeer {
        instance_id,
        device_name,
        host,
        lan_port,
        version,
        last_seen: chrono::Utc::now(),
    });
}
