//! Persisted dashboard bind/db preferences (`~/.anycode/dashboard_preferences.json`).

use crate::schema::DashboardPreferences;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub fn preferences_path() -> PathBuf {
    std::env::var("HOME")
        .map(|h| {
            PathBuf::from(h)
                .join(".anycode")
                .join("dashboard_preferences.json")
        })
        .unwrap_or_else(|_| PathBuf::from(".anycode/dashboard_preferences.json"))
}

pub fn load_preferences() -> Option<DashboardPreferences> {
    let path = preferences_path();
    let text = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_preferences(prefs: &DashboardPreferences) -> Result<PathBuf> {
    let path = preferences_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).context("create preferences directory")?;
    }
    let body = serde_json::to_string_pretty(prefs).context("serialize preferences")?;
    std::fs::write(&path, body).context("write preferences")?;
    Ok(path)
}

pub fn restart_command(host: &str, port: u16, db_path: &Path) -> String {
    format!(
        "anycode dashboard --host {host} --port {port} --db {}",
        shell_quote(db_path.display().to_string())
    )
}

fn shell_quote(s: String) -> String {
    if s.chars()
        .any(|c| c.is_whitespace() || c == '\'' || c == '"')
    {
        format!("'{s}'")
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restart_command_quotes_paths_with_spaces() {
        let cmd = restart_command("127.0.0.1", 43180, Path::new("/tmp/my db/projects.db"));
        assert!(cmd.contains("'"));
    }
}
