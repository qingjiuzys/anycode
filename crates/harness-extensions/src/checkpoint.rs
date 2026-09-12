use anycode_harness_core::{Error, Result};
use fs2::FileExt;
use serde_json::Value;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

pub trait StoreLease: Send {}
impl StoreLease for tokio::sync::OwnedMutexGuard<()> {}
pub trait CheckpointStore: Send + Sync {
    /// An operation-wide in-process lease; file stores additionally hold an OS lease.
    fn lease(&self) -> Result<Box<dyn StoreLease>>;
    fn load(&self) -> Result<Option<Value>>;
    fn save(&self, value: &Value) -> Result<()>;
}
#[derive(Default)]
pub struct MemoryCheckpoint {
    value: Mutex<Option<Value>>,
    execution: Arc<tokio::sync::Mutex<()>>,
}
impl CheckpointStore for MemoryCheckpoint {
    fn lease(&self) -> Result<Box<dyn StoreLease>> {
        Ok(Box::new(self.execution.clone().try_lock_owned().map_err(
            |_| Error::Conflict("checkpoint operation active".into()),
        )?))
    }
    fn load(&self) -> Result<Option<Value>> {
        Ok(self
            .value
            .lock()
            .map_err(|_| Error::Host("checkpoint lock".into()))?
            .clone())
    }
    fn save(&self, value: &Value) -> Result<()> {
        *self
            .value
            .lock()
            .map_err(|_| Error::Host("checkpoint lock".into()))? = Some(value.clone());
        Ok(())
    }
}
/// Caller supplies a host-owned path, NEVER an HTTP/LLM filesystem argument.
/// One lease must live for the entire graph run, including all saves.
pub struct FileCheckpoint {
    path: PathBuf,
    _lease: File,
    io_lock: Mutex<()>,
    execution: Arc<tokio::sync::Mutex<()>>,
}
impl FileCheckpoint {
    pub fn open(path: &Path) -> Result<Self> {
        let parent = path
            .parent()
            .ok_or_else(|| Error::Invalid("checkpoint parent".into()))?;
        if !parent.is_dir() {
            return Err(Error::Invalid(
                "provision private checkpoint directory first".into(),
            ));
        }
        for target in [path.to_path_buf(), path.with_extension("lock")] {
            if target.exists() && std::fs::symlink_metadata(target)?.file_type().is_symlink() {
                return Err(Error::Denied("checkpoint symlink".into()));
            }
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lease = options.open(path.with_extension("lock"))?;
        FileExt::try_lock_exclusive(&lease)
            .map_err(|_| Error::Conflict("graph is already leased".into()))?;
        Ok(Self {
            path: path.into(),
            _lease: lease,
            io_lock: Mutex::new(()),
            execution: Arc::new(tokio::sync::Mutex::new(())),
        })
    }
}
impl CheckpointStore for FileCheckpoint {
    fn lease(&self) -> Result<Box<dyn StoreLease>> {
        Ok(Box::new(self.execution.clone().try_lock_owned().map_err(
            |_| Error::Conflict("checkpoint operation active".into()),
        )?))
    }
    fn load(&self) -> Result<Option<Value>> {
        let _lock = self
            .io_lock
            .lock()
            .map_err(|_| Error::Host("checkpoint lock".into()))?;
        if !self.path.exists() {
            return Ok(None);
        }
        if std::fs::metadata(&self.path)?.len() > 16 * 1024 * 1024 {
            return Err(Error::Invalid("checkpoint too large".into()));
        }
        // Corrupt JSON is a hard error, never silently a new workflow.
        Ok(Some(serde_json::from_slice(&std::fs::read(&self.path)?)?))
    }
    fn save(&self, value: &Value) -> Result<()> {
        let _lock = self
            .io_lock
            .lock()
            .map_err(|_| Error::Host("checkpoint lock".into()))?;
        let bytes = serde_json::to_vec(value)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::Invalid("checkpoint too large".into()));
        }
        let temporary = self.path.with_extension(format!("{}.tmp", Uuid::new_v4()));
        let result = (|| -> Result<()> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            std::fs::rename(&temporary, &self.path)?;
            #[cfg(unix)]
            {
                File::open(
                    self.path
                        .parent()
                        .ok_or_else(|| Error::Invalid("parent".into()))?,
                )?
                .sync_all()?;
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checkpoint_is_exclusive_and_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cp.json");
        {
            let store = FileCheckpoint::open(&path).unwrap();
            store.save(&serde_json::json!({"revision":1})).unwrap();
            assert!(FileCheckpoint::open(&path).is_err());
        }
        assert_eq!(
            FileCheckpoint::open(&path)
                .unwrap()
                .load()
                .unwrap()
                .unwrap()["revision"],
            1
        );
    }
    #[test]
    fn corrupt_checkpoint_does_not_start_over() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("cp.json");
        std::fs::write(&path, "{").unwrap();
        assert!(FileCheckpoint::open(&path).unwrap().load().is_err());
    }
}
