use crate::{digest::digest, events::Event, Error, Result};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashSet},
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::Mutex,
};
use uuid::Uuid;

const MAX_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;
const MAX_RECORD_BYTES: usize = 8 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub sequence: u64,
    pub previous_hash: String,
    pub event: Event,
    pub hash: String,
}
fn hash_record(sequence: u64, previous_hash: &str, event: &Event) -> Result<String> {
    digest(&(sequence, previous_hash, event))
}
pub trait EventSink: Send + Sync {
    /// Implementations must durably append (or return an error) and reject duplicate
    /// run_start IDs. In-memory implementation is only for tests/local prototypes.
    fn append(&self, event: Event) -> Result<Record>;
}
#[derive(Default)]
struct State {
    records: Vec<Record>,
    runs: HashSet<Uuid>,
    poisoned: bool,
}
impl State {
    fn candidate(&self, event: Event) -> Result<Record> {
        if self.poisoned {
            return Err(Error::Uncertain("journal needs repair".into()));
        }
        if event.kind == "run_start" && self.runs.contains(&event.run_id) {
            return Err(Error::Conflict(
                "run ID already started; create a new run, do not replay tools".into(),
            ));
        }
        let sequence = self.records.len() as u64 + 1;
        let previous_hash = self
            .records
            .last()
            .map(|r| r.hash.clone())
            .unwrap_or_default();
        let hash = hash_record(sequence, &previous_hash, &event)?;
        Ok(Record {
            sequence,
            previous_hash,
            event,
            hash,
        })
    }
    fn commit(&mut self, record: Record) {
        if record.event.kind == "run_start" {
            self.runs.insert(record.event.run_id);
        }
        self.records.push(record);
    }
}
#[derive(Default)]
pub struct MemoryJournal(Mutex<State>);
impl MemoryJournal {
    pub fn records(&self) -> Result<Vec<Record>> {
        Ok(self
            .0
            .lock()
            .map_err(|_| Error::Host("journal lock".into()))?
            .records
            .clone())
    }
}
impl EventSink for MemoryJournal {
    fn append(&self, event: Event) -> Result<Record> {
        let mut state = self
            .0
            .lock()
            .map_err(|_| Error::Host("journal lock".into()))?;
        let record = state.candidate(event)?;
        state.commit(record.clone());
        Ok(record)
    }
}
struct FileState {
    file: File,
    state: State,
}
/// Single-process AND cross-process writer lease. Hash chain detects corruption,
/// not malicious rewriting by an administrator with access to the whole file.
/// The host must provision a private, non-shared journal directory (0700 on Unix).
pub struct FileJournal(Mutex<FileState>);
impl FileJournal {
    pub fn open(path: &Path) -> Result<Self> {
        if path.exists() && std::fs::symlink_metadata(path)?.file_type().is_symlink() {
            return Err(Error::Denied("journal symlink".into()));
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(path)?;
        FileExt::try_lock_exclusive(&file)
            .map_err(|_| Error::Conflict("journal writer already active".into()))?;
        if file.metadata()?.len() > MAX_JOURNAL_BYTES {
            return Err(Error::Invalid(
                "journal size limit; rotate before continuing".into(),
            ));
        }
        let mut bytes = vec![];
        file.read_to_end(&mut bytes)?;
        if !bytes.is_empty() && bytes.last() != Some(&b'\n') {
            return Err(Error::Uncertain(
                "torn journal tail; preserve bytes and reconcile, never silently truncate".into(),
            ));
        }
        let mut state = State::default();
        for line in bytes.split(|b| *b == b'\n').filter(|line| !line.is_empty()) {
            if line.len() > MAX_RECORD_BYTES {
                return Err(Error::Invalid("record size limit".into()));
            }
            let record: Record = serde_json::from_slice(line)?;
            let expected = state.candidate(record.event.clone())?;
            if record.sequence != expected.sequence
                || record.previous_hash != expected.previous_hash
                || record.hash != expected.hash
            {
                return Err(Error::Uncertain("journal sequence/hash mismatch".into()));
            }
            state.commit(record);
        }
        file.seek(SeekFrom::End(0))?;
        Ok(Self(Mutex::new(FileState { file, state })))
    }
    pub fn replay_after(&self, cursor: u64, scope_digest: &str) -> Result<Vec<Record>> {
        let inner = self
            .0
            .lock()
            .map_err(|_| Error::Host("journal lock".into()))?;
        Ok(inner
            .state
            .records
            .iter()
            .filter(|r| r.sequence > cursor && r.event.scope_digest == scope_digest)
            .cloned()
            .collect())
    }
    pub fn unresolved_intents(&self) -> Result<BTreeSet<String>> {
        let inner = self
            .0
            .lock()
            .map_err(|_| Error::Host("journal lock".into()))?;
        let mut pending = BTreeSet::new();
        for record in &inner.state.records {
            let Some(id) = record.event.data["invocation_id"].as_str() else {
                continue;
            };
            let key = format!("{}:{id}", record.event.run_id);
            match record.event.kind.as_str() {
                "tool_intent" => {
                    pending.insert(key);
                }
                "tool_end" => {
                    pending.remove(&key);
                }
                _ => {}
            }
        }
        Ok(pending)
    }
}
impl EventSink for FileJournal {
    fn append(&self, event: Event) -> Result<Record> {
        let mut inner = self
            .0
            .lock()
            .map_err(|_| Error::Host("journal lock".into()))?;
        let record = inner.state.candidate(event)?;
        let mut bytes = serde_json::to_vec(&record)?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(Error::Invalid("event too large".into()));
        }
        bytes.push(b'\n');
        if inner
            .file
            .metadata()?
            .len()
            .saturating_add(bytes.len() as u64)
            > MAX_JOURNAL_BYTES
        {
            return Err(Error::Invalid(
                "journal full; explicit rotation required".into(),
            ));
        }
        // A partially written tail poisons this handle. Never append behind it.
        if let Err(error) = inner
            .file
            .write_all(&bytes)
            .and_then(|_| inner.file.sync_all())
        {
            inner.state.poisoned = true;
            return Err(error.into());
        }
        inner.state.commit(record.clone());
        Ok(record)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn event(kind: &str, run: Uuid) -> Event {
        Event {
            version: 1,
            run_id: run,
            root_id: run,
            parent_run_id: None,
            scope_digest: "s".into(),
            kind: kind.into(),
            data: serde_json::json!({}),
        }
    }
    #[test]
    fn persistence_replay_and_single_writer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        let run = Uuid::new_v4();
        {
            let journal = FileJournal::open(&path).unwrap();
            journal.append(event("run_start", run)).unwrap();
            assert!(FileJournal::open(&path).is_err());
        }
        let journal = FileJournal::open(&path).unwrap();
        assert_eq!(journal.replay_after(0, "s").unwrap().len(), 1);
        assert!(journal.replay_after(0, "other").unwrap().is_empty());
        assert!(journal.append(event("run_start", run)).is_err());
    }
    #[test]
    fn torn_tail_is_not_silently_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("events.jsonl");
        std::fs::write(&path, b"{partial").unwrap();
        assert!(matches!(FileJournal::open(&path), Err(Error::Uncertain(_))));
    }
}
