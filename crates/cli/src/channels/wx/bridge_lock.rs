//! WeChat bridge process lock — proactive outbound detects a live bridge without calling getupdates.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeLock {
    pub pid: u32,
    pub started_at_ms: u64,
}

pub fn bridge_lock_path(data_root: &Path) -> PathBuf {
    data_root.join("bridge.lock")
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn write_bridge_lock(data_root: &Path) -> Result<()> {
    std::fs::create_dir_all(data_root)?;
    let lock = BridgeLock {
        pid: std::process::id(),
        started_at_ms: now_ms(),
    };
    let path = bridge_lock_path(data_root);
    let raw = serde_json::to_string_pretty(&lock)?;
    std::fs::write(&path, raw + "\n").with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn clear_bridge_lock(data_root: &Path) {
    let _ = std::fs::remove_file(bridge_lock_path(data_root));
}

fn process_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}

/// True when `bridge.lock` exists and its PID is still running.
pub fn wechat_bridge_active(data_root: &Path) -> bool {
    let path = bridge_lock_path(data_root);
    let Ok(raw) = std::fs::read_to_string(&path) else {
        return false;
    };
    let Ok(lock) = serde_json::from_str::<BridgeLock>(&raw) else {
        return false;
    };
    process_alive(lock.pid)
}

pub struct BridgeLockGuard {
    data_root: PathBuf,
}

impl BridgeLockGuard {
    pub fn acquire(data_root: &Path) -> Result<Self> {
        write_bridge_lock(data_root)?;
        Ok(Self {
            data_root: data_root.to_path_buf(),
        })
    }
}

impl Drop for BridgeLockGuard {
    fn drop(&mut self) {
        clear_bridge_lock(&self.data_root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bridge_lock_roundtrip() {
        let dir =
            std::env::temp_dir().join(format!("anycode-wx-bridge-lock-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        {
            let _guard = BridgeLockGuard::acquire(&dir).unwrap();
            assert!(bridge_lock_path(&dir).is_file());
            assert!(wechat_bridge_active(&dir));
        }
        assert!(!wechat_bridge_active(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
