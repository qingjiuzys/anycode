//! Project-scoped filesystem listing and read for the workbench Files panel.

use super::path_guard::{
    relative_path, resolve_project_root, resolve_under_root, should_skip_entry,
};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::path::Path;

pub const DEFAULT_MAX_READ_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FsEntryKind {
    File,
    Dir,
    Symlink,
}

#[derive(Debug, Clone, Serialize)]
pub struct FsEntry {
    pub name: String,
    pub path: String,
    pub kind: FsEntryKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FsStat {
    pub path: String,
    pub kind: FsEntryKind,
    pub size: u64,
    pub modified: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FsReadResult {
    pub path: String,
    pub content: String,
    pub truncated: bool,
    pub size: u64,
    pub mime_hint: String,
}

pub fn list_dir(root_path: &str, rel: &str) -> Result<Vec<FsEntry>> {
    let root = resolve_project_root(root_path)?;
    let dir = resolve_under_root(&root, rel)?;
    if !dir.is_dir() {
        bail!("not a directory");
    }
    let mut entries = Vec::new();
    for ent in std::fs::read_dir(&dir).with_context(|| format!("read_dir {}", dir.display()))? {
        let ent = ent?;
        let ft = ent.file_type()?;
        let name = ent.file_name().to_string_lossy().into_owned();
        if should_skip_entry(&name, ft.is_dir()) {
            continue;
        }
        let kind = if ft.is_dir() {
            FsEntryKind::Dir
        } else if ft.is_symlink() {
            FsEntryKind::Symlink
        } else {
            FsEntryKind::File
        };
        let path = relative_path(&root, &ent.path());
        let size = if ft.is_file() {
            ent.metadata().ok().map(|m| m.len())
        } else {
            None
        };
        entries.push(FsEntry {
            name,
            path,
            kind,
            size,
        });
    }
    entries.sort_by(|a, b| {
        let a_dir = matches!(a.kind, FsEntryKind::Dir);
        let b_dir = matches!(b.kind, FsEntryKind::Dir);
        b_dir
            .cmp(&a_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

pub fn stat_path(root_path: &str, rel: &str) -> Result<FsStat> {
    let root = resolve_project_root(root_path)?;
    let path = resolve_under_root(&root, rel)?;
    let meta = std::fs::metadata(&path).with_context(|| format!("stat {}", path.display()))?;
    let kind = if meta.is_dir() {
        FsEntryKind::Dir
    } else if meta.file_type().is_symlink() {
        FsEntryKind::Symlink
    } else {
        FsEntryKind::File
    };
    let modified = meta
        .modified()
        .ok()
        .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339());
    Ok(FsStat {
        path: relative_path(&root, &path),
        kind,
        size: meta.len(),
        modified,
    })
}

pub fn read_file(root_path: &str, rel: &str, max_bytes: usize) -> Result<FsReadResult> {
    let root = resolve_project_root(root_path)?;
    let path = resolve_under_root(&root, rel)?;
    if path.is_dir() {
        bail!("cannot read directory as file");
    }
    let meta = std::fs::metadata(&path)?;
    let size = meta.len();
    if is_probably_binary(&path) {
        bail!("binary file not supported for preview");
    }
    let cap = max_bytes.max(1024).min(DEFAULT_MAX_READ_BYTES);
    let truncated = size as usize > cap;
    let bytes = if truncated {
        read_prefix(&path, cap)?
    } else {
        std::fs::read(&path)?
    };
    let content = String::from_utf8(bytes).context("file is not valid UTF-8")?;
    Ok(FsReadResult {
        path: relative_path(&root, &path),
        content,
        truncated,
        size,
        mime_hint: mime_hint_for_path(&path),
    })
}

fn read_prefix(path: &Path, max: usize) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; max];
    let n = file.read(&mut buf)?;
    buf.truncate(n);
    Ok(buf)
}

fn is_probably_binary(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    matches!(
        ext.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "webp"
            | "ico"
            | "pdf"
            | "zip"
            | "gz"
            | "tar"
            | "wasm"
            | "exe"
            | "dll"
            | "so"
            | "dylib"
            | "mp3"
            | "mp4"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "icns"
    )
}

fn mime_hint_for_path(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "rs" => "text/x-rust",
        "ts" | "tsx" => "text/typescript",
        "js" | "jsx" => "text/javascript",
        "json" => "application/json",
        "md" => "text/markdown",
        "html" => "text/html",
        "css" => "text/css",
        "yaml" | "yml" => "text/yaml",
        "toml" => "text/toml",
        "sh" => "text/x-shellscript",
        "py" => "text/x-python",
        _ => "text/plain",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn list_and_read_text_file() {
        let dir = tempdir().unwrap();
        let root = dir.path().join("proj");
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        let root_str = root.to_string_lossy();
        let entries = list_dir(&root_str, "").unwrap();
        assert!(entries.iter().any(|e| e.name == "src"));
        let read = read_file(&root_str, "src/main.rs", 4096).unwrap();
        assert!(read.content.contains("fn main"));
    }
}
