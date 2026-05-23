//! Local Digital Workbench: Axum API + optional static UI.

use crate::cli_args::{
    DashboardDbCommands, DashboardRunArgs, DashboardSubcommands, DashboardTokenCommands,
};
use anycode_dashboard::{
    backup_db, create_token, db_operations, default_db_path, discover_ui_dist, list_tokens,
    revoke_token, run, run_doctor_checks, DashboardConfig, DashboardDb,
};
use anyhow::{Context, Result};
use std::path::PathBuf;

pub async fn run_dashboard_command(
    sub: Option<DashboardSubcommands>,
    run: DashboardRunArgs,
) -> Result<()> {
    match sub {
        None => {
            let prefs = anycode_dashboard::load_preferences();
            let host = run
                .host
                .or_else(|| prefs.as_ref().map(|p| p.host.clone()))
                .unwrap_or_else(|| "127.0.0.1".into());
            let port = run
                .port
                .or_else(|| prefs.as_ref().map(|p| p.port))
                .unwrap_or(43_180);
            let db = run
                .db
                .or_else(|| prefs.as_ref().map(|p| PathBuf::from(&p.db_path)));
            run_dashboard(host, port, db, run.static_dir, run.open).await
        }
        Some(DashboardSubcommands::Status { db, json }) => dashboard_status(db, json).await,
        Some(DashboardSubcommands::Doctor {
            db,
            host,
            port,
            static_dir,
            json,
        }) => dashboard_doctor(db, host, port, static_dir, json).await,
        Some(DashboardSubcommands::Token { sub }) => dashboard_token(sub).await,
        Some(DashboardSubcommands::Db { sub }) => dashboard_db(sub).await,
    }
}

pub async fn run_dashboard(
    host: String,
    port: u16,
    db_path: Option<PathBuf>,
    static_dir: Option<PathBuf>,
    open_browser: bool,
) -> Result<()> {
    let paths = load_workspace_paths();
    let static_dir = static_dir.or_else(discover_ui_dist);
    let config = DashboardConfig {
        host: host.clone(),
        port,
        db_path: db_path.unwrap_or_else(default_db_path),
        static_dir: static_dir.clone(),
        version: env!("CARGO_PKG_VERSION").into(),
    };
    let url = format!("http://{host}:{port}/");
    let loopback = host == "127.0.0.1" || host == "localhost" || host == "::1";
    if !loopback {
        eprintln!(
            "Warning: dashboard bound to {host} (not loopback). Create an API token: anycode dashboard token create"
        );
    }
    if open_browser {
        let _ = try_open_browser(&url);
    } else {
        println!("Digital Workbench: {url}");
        println!("API: {url}api/health");
        println!("Doctor: anycode dashboard doctor");
        println!("Reports: {url}reports");
        println!("Audit: {url}audit");
        println!("DB: {}", config.db_path.display());
        println!("Planning (V3): docs/digital-workbench-STATUS.md · docs/digital-workbench-next-steps-zh.md");
        if let Some(ref ui) = static_dir {
            println!("UI: {} (bundled)", ui.display());
        } else if anycode_dashboard::embedded_ui::available() {
            println!("UI: embedded in binary (release build)");
        } else {
            println!(
                "UI dev: cd crates/dashboard-ui && npm ci && npm run build && anycode dashboard"
            );
            println!("     or: cd crates/dashboard-ui && npm run dev  (proxies API to :43180)");
        }
    }
    run(config, paths).await.context("dashboard server")
}

async fn dashboard_status(db_path: Option<PathBuf>, json: bool) -> Result<()> {
    let path = db_path.unwrap_or_else(default_db_path);
    let static_dir = discover_ui_dist();
    let doctor = run_doctor_checks("127.0.0.1", 43_180, &path, static_dir.as_deref());
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "db_path": path.display().to_string(),
                "ui_dist": static_dir.as_ref().map(|p| p.display().to_string()),
                "doctor": doctor,
            }))?
        );
    } else {
        println!("DB: {}", path.display());
        if let Some(ui) = static_dir {
            println!("UI dist: {}", ui.display());
        } else {
            println!("UI dist: not found");
        }
        println!("Doctor: {}", doctor.status);
        for c in &doctor.checks {
            println!("  [{}] {} — {}", c.status, c.id, c.message);
        }
    }
    Ok(())
}

async fn dashboard_doctor(
    db_path: Option<PathBuf>,
    host: String,
    port: u16,
    static_dir: Option<PathBuf>,
    json: bool,
) -> Result<()> {
    let path = db_path.unwrap_or_else(default_db_path);
    let static_dir = static_dir.or_else(discover_ui_dist);
    let report = run_doctor_checks(&host, port, &path, static_dir.as_deref());
    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Dashboard doctor: {}", report.status);
        for c in &report.checks {
            println!("  [{}] {}", c.status, c.message);
        }
    }
    if report.status == "error" {
        std::process::exit(1);
    }
    Ok(())
}

async fn dashboard_token(sub: DashboardTokenCommands) -> Result<()> {
    let db_path = match &sub {
        DashboardTokenCommands::Create { db, .. }
        | DashboardTokenCommands::List { db, .. }
        | DashboardTokenCommands::Revoke { db, .. } => db.clone().unwrap_or_else(default_db_path),
    };
    let db = DashboardDb::open(&db_path)
        .await
        .context("open dashboard db")?;
    match sub {
        DashboardTokenCommands::Create {
            name, expires_days, ..
        } => {
            let created = create_token(&db, &name, expires_days).await?;
            println!("Token id: {}", created.record.id);
            println!("Prefix: {}", created.record.prefix);
            println!("Plaintext (save now): {}", created.plaintext);
        }
        DashboardTokenCommands::List { json, .. } => {
            let tokens = list_tokens(&db).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&tokens)?);
            } else {
                for t in tokens {
                    println!(
                        "{}  {}  prefix={}  revoked={}",
                        t.id, t.name, t.prefix, t.revoked
                    );
                }
            }
        }
        DashboardTokenCommands::Revoke { id, .. } => {
            if revoke_token(&db, &id).await? {
                println!("Revoked {id}");
            } else {
                anyhow::bail!("token not found: {id}");
            }
        }
    }
    Ok(())
}

async fn dashboard_db(sub: DashboardDbCommands) -> Result<()> {
    match sub {
        DashboardDbCommands::Check { db, json } => {
            let path = db.unwrap_or_else(default_db_path);
            let dashboard = DashboardDb::open(&path).await.context("open db")?;
            let ops = db_operations(&dashboard).await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&ops)?);
            } else {
                println!(
                    "DB: {} ({:.1} MB)",
                    ops.db_path,
                    ops.db_size_bytes as f64 / 1_048_576.0
                );
                println!("Health: {}", ops.health_status);
                println!("Migrations: {}", ops.migrations.len());
                for w in &ops.growth_warnings {
                    println!("  warn: {w}");
                }
                println!("Backup suggestion: {}", ops.backup_suggestion);
            }
        }
        DashboardDbCommands::Backup { db, output } => {
            let path = db.unwrap_or_else(default_db_path);
            let dest = output.unwrap_or_else(|| {
                anycode_dashboard::service_governance::suggest_backup_path(&path)
            });
            backup_db(&path, &dest).context("backup db")?;
            println!("Backed up to {}", dest.display());
        }
    }
    Ok(())
}

fn load_workspace_paths() -> Vec<String> {
    let index = crate::workspace::root().join("projects").join("index.json");
    let raw = match std::fs::read_to_string(&index) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    #[derive(serde::Deserialize)]
    struct Index {
        #[serde(default)]
        projects: Vec<Entry>,
    }
    #[derive(serde::Deserialize)]
    struct Entry {
        path: String,
    }
    serde_json::from_str::<Index>(&raw)
        .ok()
        .map(|idx| idx.projects.into_iter().map(|p| p.path).collect())
        .unwrap_or_default()
}

fn try_open_browser(url: &str) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).status()?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(url).status()?;
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = url;
    }
    Ok(())
}
