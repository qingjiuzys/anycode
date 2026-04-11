//! 虚态缓冲 WAL：JSONL append，进程重启后重放。

use anycode_core::PreSemanticFragment;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use tracing::warn;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "op", rename_all = "lowercase")]
enum WalLine {
    Put { f: PreSemanticFragment },
    Del { id: String },
}

pub struct BufferWal {
    #[allow(dead_code)]
    path: PathBuf,
    inner: Mutex<WalInner>,
    fsync_every_n: u32,
    writes: AtomicU32,
}

struct WalInner {
    file: File,
}

impl BufferWal {
    pub fn open(path: impl Into<PathBuf>, fsync_every_n: u32) -> std::io::Result<Self> {
        let path = path.into();
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        Ok(Self {
            path,
            inner: Mutex::new(WalInner { file }),
            fsync_every_n: fsync_every_n.max(1),
            writes: AtomicU32::new(0),
        })
    }

    /// 重放 WAL，返回缓冲快照。
    pub fn replay(
        path: &Path,
    ) -> std::io::Result<std::collections::HashMap<String, PreSemanticFragment>> {
        let mut map = std::collections::HashMap::new();
        if !path.exists() {
            return Ok(map);
        }
        let f = File::open(path)?;
        for (line_no, line) in BufReader::new(f).lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    warn!(target: "anycode_memory", "wal line {} read err: {}", line_no, e);
                    continue;
                }
            };
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            match serde_json::from_str::<WalLine>(line) {
                Ok(WalLine::Put { f }) => {
                    map.insert(f.id.clone(), f);
                }
                Ok(WalLine::Del { id }) => {
                    map.remove(&id);
                }
                Err(e) => warn!(target: "anycode_memory", "wal line {} parse: {}", line_no, e),
            }
        }
        Ok(map)
    }

    pub fn append_put(&self, frag: &PreSemanticFragment) -> std::io::Result<()> {
        let json = serde_json::to_string(&WalLine::Put { f: frag.clone() })?;
        self.append_line(&json)
    }

    pub fn append_delete(&self, id: &str) -> std::io::Result<()> {
        let json = serde_json::to_string(&WalLine::Del { id: id.to_string() })?;
        self.append_line(&json)
    }

    fn append_line(&self, line: &str) -> std::io::Result<()> {
        let mut g = self.inner.lock().expect("wal mutex");
        writeln!(g.file, "{}", line)?;
        let n = self.writes.fetch_add(1, Ordering::SeqCst) + 1;
        if n.is_multiple_of(self.fsync_every_n) {
            g.file.sync_all()?;
        }
        Ok(())
    }

    /// 进程退出或显式 checkpoint 时刷盘。
    pub fn sync_all(&self) -> std::io::Result<()> {
        let g = self.inner.lock().expect("wal mutex");
        g.file.sync_all()
    }

    #[allow(dead_code)] // 调试 / 未来诊断路径
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anycode_core::MemoryType;
    use chrono::Utc;

    #[test]
    fn replay_roundtrip() {
        let p = std::env::temp_dir().join(format!("wal-{}.jsonl", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_file(&p);
        let wal = BufferWal::open(&p, 1).unwrap();
        let f1 = PreSemanticFragment {
            id: "a".to_string(),
            session_id: "s".to_string(),
            mem_type: MemoryType::Project,
            raw_text: "x".to_string(),
            created_at: Utc::now(),
            last_touched_at: Utc::now(),
            touch_count: 1,
        };
        wal.append_put(&f1).unwrap();
        wal.append_delete("a").unwrap();
        let f2 = PreSemanticFragment {
            id: "b".to_string(),
            session_id: "s".to_string(),
            mem_type: MemoryType::Project,
            raw_text: "y".to_string(),
            created_at: Utc::now(),
            last_touched_at: Utc::now(),
            touch_count: 2,
        };
        wal.append_put(&f2).unwrap();
        drop(wal);

        let m = BufferWal::replay(&p).unwrap();
        assert!(!m.contains_key("a"));
        assert_eq!(m.get("b").map(|x| x.raw_text.as_str()), Some("y"));
    }

    /// 高 `fsync_every_n` 下中间不会刷盘；显式 `sync_all` 后重放可见。
    #[test]
    fn explicit_sync_persists_when_periodic_fsync_skipped() {
        let p = std::env::temp_dir().join(format!("wal-fsync-{}.jsonl", uuid::Uuid::new_v4()));
        let _ = std::fs::remove_file(&p);
        let wal = BufferWal::open(&p, 10_000).unwrap();
        let f = PreSemanticFragment {
            id: "sparse".to_string(),
            session_id: "s".to_string(),
            mem_type: MemoryType::Project,
            raw_text: "payload".to_string(),
            created_at: Utc::now(),
            last_touched_at: Utc::now(),
            touch_count: 1,
        };
        wal.append_put(&f).unwrap();
        wal.sync_all().unwrap();
        drop(wal);
        let m = BufferWal::replay(&p).unwrap();
        assert_eq!(
            m.get("sparse").map(|x| x.raw_text.as_str()),
            Some("payload")
        );
    }
}
