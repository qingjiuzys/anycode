//! Single-node durable run store. File CAS and leases are not a cluster database.
use crate::{budget::BudgetSnapshot, digest::bytes_digest, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeLease {
    pub node_id: String,
    pub token: Uuid,
    pub fence: u64,
    pub until_unix: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct RunRecord {
    pub run_id: Uuid,
    pub revision: u64,
    pub fence: u64,
    pub node_lease: Option<NodeLease>,
    pub budget: BudgetSnapshot,
    pub payload: Value,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct OutboxRecord {
    pub sequence: u64,
    pub kind: String,
    pub payload: Value,
    pub idempotency_key: String,
    pub digest: String,
}

/// Host-owned directory store. Callers provision the root; LLM/HTTP paths are refused.
pub struct FileRunStore {
    root: PathBuf,
}

impl FileRunStore {
    pub fn open(root: &Path) -> Result<Self> {
        if !root.is_absolute() {
            return Err(Error::Invalid("run store root must be absolute".into()));
        }
        if root.exists() && std::fs::symlink_metadata(root)?.file_type().is_symlink() {
            return Err(Error::Denied("run store root symlink".into()));
        }
        std::fs::create_dir_all(root)?;
        let root = std::fs::canonicalize(root)?;
        Ok(Self { root })
    }

    fn run_dir(&self, run_id: Uuid) -> PathBuf {
        self.root.join(run_id.to_string())
    }

    fn record_path(&self, run_id: Uuid) -> PathBuf {
        self.run_dir(run_id).join("record.json")
    }

    fn outbox_path(&self, run_id: Uuid) -> PathBuf {
        self.run_dir(run_id).join("outbox.json")
    }

    fn reject_symlink(path: &Path) -> Result<()> {
        if path.exists() && std::fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(Error::Denied("run store symlink".into()));
        }
        Ok(())
    }

    fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
        Self::reject_symlink(path)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::Invalid("run record too large".into()));
        }
        let parent = path
            .parent()
            .ok_or_else(|| Error::Invalid("run record parent".into()))?;
        std::fs::create_dir_all(parent)?;
        let temporary = parent.join(format!(".{}.tmp", Uuid::new_v4()));
        let result = (|| -> Result<()> {
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temporary)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            std::fs::rename(&temporary, path)?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result
    }

    fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
        Self::reject_symlink(path)?;
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err(Error::Invalid("run record too large".into()));
        }
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub fn create(&self, record: &RunRecord) -> Result<()> {
        let dir = self.run_dir(record.run_id);
        if dir.exists() {
            return Err(Error::Conflict("run already exists".into()));
        }
        std::fs::create_dir_all(&dir)?;
        if record.revision != 0 {
            return Err(Error::Invalid("new run revision must be 0".into()));
        }
        Self::atomic_write(
            &self.record_path(record.run_id),
            &serde_json::to_vec(record)?,
        )?;
        Self::atomic_write(&self.outbox_path(record.run_id), b"[]")?;
        Ok(())
    }

    pub fn load(&self, run_id: Uuid) -> Result<RunRecord> {
        let path = self.record_path(run_id);
        if !path.exists() {
            return Err(Error::Invalid("run not found".into()));
        }
        Self::read_json(&path)
    }

    /// Compare-and-swap on `revision`. The next record must be `expected + 1`.
    pub fn cas(&self, expected_revision: u64, next: &RunRecord) -> Result<RunRecord> {
        let current = self.load(next.run_id)?;
        if current.revision != expected_revision || next.revision != expected_revision + 1 {
            return Err(Error::Conflict("run revision CAS failed".into()));
        }
        if next.fence < current.fence {
            return Err(Error::Denied("stale fencing token".into()));
        }
        Self::atomic_write(&self.record_path(next.run_id), &serde_json::to_vec(next)?)?;
        Ok(next.clone())
    }

    pub fn acquire_node_lease(
        &self,
        run_id: Uuid,
        node_id: &str,
        ttl_secs: u64,
    ) -> Result<NodeLease> {
        if node_id.is_empty() || node_id.len() > 64 || ttl_secs == 0 || ttl_secs > 300 {
            return Err(Error::Invalid("node lease bounds".into()));
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::Host("clock".into()))?
            .as_secs();
        let current = self.load(run_id)?;
        if let Some(existing) = &current.node_lease {
            if existing.until_unix > now {
                return Err(Error::Conflict("node already leased".into()));
            }
        }
        let lease = NodeLease {
            node_id: node_id.into(),
            token: Uuid::new_v4(),
            fence: current.fence.saturating_add(1),
            until_unix: now.saturating_add(ttl_secs),
        };
        let mut next = current;
        let expected = next.revision;
        next.revision = expected + 1;
        next.fence = lease.fence;
        next.node_lease = Some(lease.clone());
        self.cas(expected, &next)?;
        Ok(lease)
    }

    pub fn release_node_lease(&self, run_id: Uuid, token: Uuid) -> Result<()> {
        let current = self.load(run_id)?;
        let Some(lease) = &current.node_lease else {
            return Err(Error::Conflict("no node lease".into()));
        };
        if lease.token != token {
            return Err(Error::Denied("lease token mismatch".into()));
        }
        let mut next = current;
        let expected = next.revision;
        next.revision = expected + 1;
        next.node_lease = None;
        self.cas(expected, &next)?;
        Ok(())
    }

    pub fn append_outbox(
        &self,
        run_id: Uuid,
        kind: &str,
        payload: Value,
        idempotency_key: &str,
    ) -> Result<OutboxRecord> {
        if kind.is_empty()
            || kind.len() > 64
            || idempotency_key.is_empty()
            || idempotency_key.len() > 128
        {
            return Err(Error::Invalid("outbox bounds".into()));
        }
        let path = self.outbox_path(run_id);
        let mut records: Vec<OutboxRecord> = if path.exists() {
            Self::read_json(&path)?
        } else {
            Vec::new()
        };
        if let Some(existing) = records
            .iter()
            .find(|r| r.idempotency_key == idempotency_key)
        {
            if existing.kind != kind || existing.payload != payload {
                return Err(Error::Conflict(
                    "outbox idempotency payload mismatch".into(),
                ));
            }
            return Ok(existing.clone());
        }
        let record = OutboxRecord {
            sequence: records.last().map(|r| r.sequence + 1).unwrap_or(1),
            kind: kind.into(),
            digest: bytes_digest(&serde_json::to_vec(&payload)?),
            payload,
            idempotency_key: idempotency_key.into(),
        };
        records.push(record.clone());
        Self::atomic_write(&path, &serde_json::to_vec(&records)?)?;
        Ok(record)
    }

    pub fn outbox(&self, run_id: Uuid) -> Result<Vec<OutboxRecord>> {
        let path = self.outbox_path(run_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        Self::read_json(&path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::budget::BudgetSnapshot;

    fn record(run_id: Uuid) -> RunRecord {
        RunRecord {
            run_id,
            revision: 0,
            fence: 0,
            node_lease: None,
            budget: BudgetSnapshot {
                limit: 100,
                spent: 0,
                reserved: 0,
                overdrawn: false,
            },
            payload: serde_json::json!({"status":"pending"}),
        }
    }

    #[test]
    fn cas_rejects_stale_revision_and_replay_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileRunStore::open(&dir.path().canonicalize().unwrap()).unwrap();
        let run = Uuid::new_v4();
        store.create(&record(run)).unwrap();
        let mut next = store.load(run).unwrap();
        next.revision = 1;
        next.payload = serde_json::json!({"status":"running"});
        store.cas(0, &next).unwrap();
        assert!(store.cas(0, &next).is_err());
        let first = store
            .append_outbox(run, "usage", serde_json::json!({"tokens":3}), "usage-1")
            .unwrap();
        let again = store
            .append_outbox(run, "usage", serde_json::json!({"tokens":3}), "usage-1")
            .unwrap();
        assert_eq!(first, again);
        assert_eq!(store.outbox(run).unwrap().len(), 1);
        assert!(store
            .append_outbox(run, "usage", serde_json::json!({"tokens":9}), "usage-1")
            .is_err());
    }

    #[test]
    fn node_lease_is_exclusive_until_released() {
        let dir = tempfile::tempdir().unwrap();
        let store = FileRunStore::open(&dir.path().canonicalize().unwrap()).unwrap();
        let run = Uuid::new_v4();
        store.create(&record(run)).unwrap();
        let lease = store.acquire_node_lease(run, "work", 30).unwrap();
        assert!(store.acquire_node_lease(run, "work", 30).is_err());
        store.release_node_lease(run, lease.token).unwrap();
        let again = store.acquire_node_lease(run, "work", 30).unwrap();
        assert!(again.fence > lease.fence);
    }
}
