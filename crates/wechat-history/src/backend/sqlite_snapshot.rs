//! Copy WeChat SQLite sidecars into a temp dir before sqlcipher opens them.
//!
//! Never pass live `db_storage` paths to `sqlcipher`; always query a snapshot copy.

use crate::{Result, WechatHistoryError};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

const COPY_RETRIES: u32 = 2;
const RETRY_DELAY: Duration = Duration::from_millis(50);

/// A read-only snapshot of a SQLite database and its `-wal` / `-shm` sidecars.
pub struct SnapshotDb {
    copy_path: PathBuf,
    _tempdir: TempDir,
}

impl SnapshotDb {
    /// Copy `original` and sibling sidecars into a fresh temp directory.
    pub fn create(original: &Path) -> Result<Self> {
        let tempdir = TempDir::new().map_err(snapshot_err)?;
        Self::create_in_dir(original, tempdir)
    }

    fn create_in_dir(original: &Path, tempdir: TempDir) -> Result<Self> {
        if !original.is_file() {
            return Err(WechatHistoryError::SqlCipher(format!(
                "snapshot source not found: {}",
                original.display()
            )));
        }
        let file_name = original.file_name().ok_or_else(|| {
            WechatHistoryError::SqlCipher(format!(
                "snapshot source has no file name: {}",
                original.display()
            ))
        })?;
        let copy_path = tempdir.path().join(file_name);
        copy_file_with_retry(original, &copy_path)?;
        for suffix in ["-wal", "-shm"] {
            copy_sidecar_if_present(original, tempdir.path(), suffix)?;
        }
        Ok(Self {
            copy_path,
            _tempdir: tempdir,
        })
    }

    pub fn copy_path(&self) -> &Path {
        &self.copy_path
    }
}

fn snapshot_err(e: impl std::fmt::Display) -> WechatHistoryError {
    WechatHistoryError::SqlCipher(format!("snapshot temp dir: {e}"))
}

fn sidecar_source(db: &Path, suffix: &str) -> Option<PathBuf> {
    let name = db.file_name()?.to_str()?;
    Some(db.with_file_name(format!("{name}{suffix}")))
}

fn copy_file_with_retry(src: &Path, dst: &Path) -> Result<()> {
    let mut last_err = None;
    for attempt in 0..COPY_RETRIES {
        if attempt > 0 {
            thread::sleep(RETRY_DELAY);
        }
        match fs::copy(src, dst) {
            Ok(_) => return Ok(()),
            Err(e) => last_err = Some(e),
        }
    }
    Err(WechatHistoryError::SqlCipher(format!(
        "copy {} -> {}: {}",
        src.display(),
        dst.display(),
        last_err.expect("retry loop")
    )))
}

fn copy_sidecar_if_present(db: &Path, dest_dir: &Path, suffix: &str) -> Result<()> {
    let Some(src) = sidecar_source(db, suffix) else {
        return Ok(());
    };
    if !src.is_file() {
        return Ok(());
    }
    let file_name = db.file_name().ok_or_else(|| {
        WechatHistoryError::SqlCipher(format!(
            "snapshot source has no file name: {}",
            db.display()
        ))
    })?;
    let dest = dest_dir.join(format!("{}{suffix}", file_name.to_string_lossy()));
    copy_file_with_retry(&src, &dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_file(path: &Path, contents: &[u8]) {
        let mut f = fs::File::create(path).unwrap();
        f.write_all(contents).unwrap();
    }

    #[test]
    fn snapshot_copies_db_wal_and_shm() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("message_0.db");
        write_file(&db, b"db-bytes");
        write_file(&db.with_file_name("message_0.db-wal"), b"wal-bytes");
        write_file(&db.with_file_name("message_0.db-shm"), b"shm-bytes");

        let snap = SnapshotDb::create(&db).unwrap();
        assert_eq!(snap.copy_path().file_name().unwrap(), "message_0.db");
        assert!(snap.copy_path().is_file());
        assert_eq!(fs::read(snap.copy_path()).unwrap(), b"db-bytes");

        let temp = snap.copy_path().parent().unwrap();
        assert_eq!(
            fs::read(temp.join("message_0.db-wal")).unwrap(),
            b"wal-bytes"
        );
        assert_eq!(
            fs::read(temp.join("message_0.db-shm")).unwrap(),
            b"shm-bytes"
        );

        // Original files must remain untouched.
        assert_eq!(fs::read(&db).unwrap(), b"db-bytes");
    }

    #[test]
    fn snapshot_works_without_sidecars() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("message_1.db");
        write_file(&db, b"only-db");

        let snap = SnapshotDb::create(&db).unwrap();
        assert_eq!(fs::read(snap.copy_path()).unwrap(), b"only-db");
        let temp = snap.copy_path().parent().unwrap();
        assert!(!temp.join("message_1.db-wal").exists());
        assert!(!temp.join("message_1.db-shm").exists());
    }

    #[test]
    fn snapshot_missing_source_errors() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.db");
        assert!(SnapshotDb::create(&missing).is_err());
    }
}
