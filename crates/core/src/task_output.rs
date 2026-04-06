use crate::error::CoreError;
use crate::ids::TaskId;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// 任务输出按任务 ID 落盘到目录（便于 tail 与审计）
#[derive(Debug, Clone)]
pub struct DiskTaskOutput {
    root_dir: PathBuf,
}

impl DiskTaskOutput {
    /// 默认落盘：~/.anycode/tasks
    pub fn new_default() -> Result<Self, CoreError> {
        let home = std::env::var("HOME").map_err(|e| anyhow::anyhow!("HOME env missing: {}", e))?;
        Ok(Self {
            root_dir: PathBuf::from(home).join(".anycode").join("tasks"),
        })
    }

    pub fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    pub fn task_dir(&self, task_id: TaskId) -> PathBuf {
        self.root_dir.join(task_id.to_string())
    }

    pub fn output_path(&self, task_id: TaskId) -> PathBuf {
        self.task_dir(task_id).join("output.log")
    }

    pub fn ensure_initialized(&self, task_id: TaskId) -> Result<PathBuf, CoreError> {
        let dir = self.task_dir(task_id);
        fs::create_dir_all(&dir)?;
        let path = self.output_path(task_id);
        if !path.exists() {
            File::create(&path)?;
        }
        Ok(path)
    }

    pub fn append(&self, task_id: TaskId, content: &str) -> Result<(), CoreError> {
        let path = self.ensure_initialized(task_id)?;
        let mut f = OpenOptions::new().create(true).append(true).open(&path)?;
        f.write_all(content.as_bytes())?;
        Ok(())
    }

    pub fn append_line(&self, task_id: TaskId, line: &str) -> Result<(), CoreError> {
        self.append(task_id, &format!("{}\n", line))
    }

    /// 读取增量（从字节 offset 开始，最多 max_bytes）
    pub fn read_delta(
        &self,
        task_id: TaskId,
        from_offset: u64,
        max_bytes: usize,
    ) -> Result<(String, u64), CoreError> {
        let path = self.output_path(task_id);
        if !path.exists() {
            return Ok((String::new(), from_offset));
        }
        let mut f = File::open(&path)?;
        let size = f.metadata()?.len();
        if from_offset >= size {
            return Ok((String::new(), from_offset));
        }
        f.seek(SeekFrom::Start(from_offset))?;

        let to_read = std::cmp::min(max_bytes as u64, size - from_offset) as usize;
        let mut buf = vec![0u8; to_read];
        f.read_exact(&mut buf)?;
        let content = String::from_utf8_lossy(&buf).to_string();
        Ok((content, from_offset + to_read as u64))
    }

    /// 读取尾部（最多 max_bytes）
    pub fn tail(&self, task_id: TaskId, max_bytes: usize) -> Result<String, CoreError> {
        let path = self.output_path(task_id);
        if !path.exists() {
            return Ok(String::new());
        }
        let mut f = File::open(&path)?;
        let size = f.metadata()?.len();
        let start = size.saturating_sub(max_bytes as u64);
        f.seek(SeekFrom::Start(start))?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf)?;
        Ok(String::from_utf8_lossy(&buf).to_string())
    }

    pub fn exists(&self, task_id: TaskId) -> bool {
        self.output_path(task_id).exists()
    }

    pub fn root_dir(&self) -> &Path {
        &self.root_dir
    }
}
