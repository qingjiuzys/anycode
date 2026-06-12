//! Local dashboard service registry: peer termination and startup coordination.

use crate::db::DashboardDb;
use crate::service_governance::{
    dashboard_allow_multi, is_dashboard_service_live, port_available, probe_dashboard_health,
    terminate_dashboard_process, wait_for_port,
};
use anyhow::Result;
use tracing::info;

#[derive(Debug, Clone)]
pub struct TerminatedPeer {
    pub host: String,
    pub port: u16,
    pub pid: Option<u32>,
}

/// Reconcile stale rows, then terminate all live dashboard peers sharing this DB.
pub async fn terminate_live_dashboard_peers(
    db: &DashboardDb,
    target_host: &str,
    target_port: u16,
) -> Result<Vec<TerminatedPeer>> {
    if dashboard_allow_multi() {
        return Ok(Vec::new());
    }

    let _ = db.reconcile_local_services("dashboard").await?;
    let rows = db.list_running_local_services("dashboard").await?;
    let mut terminated = Vec::new();

    for row in rows {
        if row.host == target_host && row.port == target_port {
            continue;
        }
        if !is_dashboard_service_live(&row.host, row.port, row.pid).await {
            continue;
        }
        if let Some(pid) = row.pid {
            eprintln!(
                "anycode: stopped previous dashboard (pid {pid}, {}:{})",
                row.host, row.port
            );
            let _ = terminate_dashboard_process(pid, 3_000);
        }
        db.mark_local_service_stopped(&row.name, &row.host, row.port)
            .await?;
        terminated.push(TerminatedPeer {
            host: row.host,
            port: row.port,
            pid: row.pid,
        });
    }

    // Target port may be held by a dashboard not yet reconciled into DB.
    if !port_available(target_host, target_port)
        && probe_dashboard_health(target_host, target_port).await
    {
        if let Some(row) = db
            .list_running_local_services("dashboard")
            .await?
            .into_iter()
            .find(|r| r.host == target_host && r.port == target_port)
        {
            if let Some(pid) = row.pid {
                eprintln!(
                    "anycode: stopped previous dashboard on target port (pid {pid}, {target_host}:{target_port})"
                );
                let _ = terminate_dashboard_process(pid, 3_000);
                db.mark_local_service_stopped(&row.name, &row.host, row.port)
                    .await?;
                terminated.push(TerminatedPeer {
                    host: row.host,
                    port: row.port,
                    pid: row.pid,
                });
            }
        }
    }

    if !terminated.is_empty() {
        wait_for_port(target_host, target_port, 5_000).await;
        info!(
            count = terminated.len(),
            "terminated previous dashboard instance(s)"
        );
    }

    Ok(terminated)
}

pub async fn mark_self_stopped(db: &DashboardDb, host: &str, port: u16) {
    if let Err(e) = db.mark_local_service_stopped("dashboard", host, port).await {
        tracing::warn!(error = %e, "failed to mark dashboard service stopped");
    }
}
