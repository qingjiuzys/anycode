//! 读文件等上限（对齐 Claude Code FileRead 的 maxSizeBytes 思路）。

pub const DEFAULT_FILE_READ_MAX_BYTES: u64 = 256 * 1024;

pub fn file_read_max_bytes() -> u64 {
    std::env::var("ANYCODE_FILE_READ_MAX_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_FILE_READ_MAX_BYTES)
}

pub const GLOB_MAX_FILES: usize = 100;
pub const GREP_MAX_JSON_LINES: usize = 800;
