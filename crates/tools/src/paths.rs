//! 沙箱路径解析（与 Claude Code cwd 语义一致）。

use crate::sandbox::{
    resolve_under_workdir, resolve_under_workdir_with_extra_read_roots,
    skill_read_roots_for_workdir,
};
use anycode_core::prelude::*;
use std::path::PathBuf;

/// 从已拆出的上下文字段解析路径，避免 `ToolInput.input` 被 move 后无法借用 `ToolInput`。
pub fn resolve_path_fields(
    tool_sandbox: bool,
    sandbox_input: bool,
    working_directory: Option<&str>,
    user_path: &str,
) -> Result<PathBuf, CoreError> {
    if tool_sandbox && sandbox_input {
        let wd = working_directory.ok_or_else(|| {
            CoreError::PermissionDenied(
                "sandbox_mode requires working_directory on tool input".to_string(),
            )
        })?;
        resolve_under_workdir(wd, user_path)
    } else {
        Ok(PathBuf::from(user_path))
    }
}

/// Read-only tools may also access skill catalog directories when sandboxed.
pub fn resolve_read_path_fields(
    tool_sandbox: bool,
    sandbox_input: bool,
    working_directory: Option<&str>,
    user_path: &str,
) -> Result<PathBuf, CoreError> {
    if tool_sandbox && sandbox_input {
        let wd = working_directory.ok_or_else(|| {
            CoreError::PermissionDenied(
                "sandbox_mode requires working_directory on tool input".to_string(),
            )
        })?;
        let extra = skill_read_roots_for_workdir(wd);
        resolve_under_workdir_with_extra_read_roots(wd, user_path, &extra)
    } else {
        Ok(PathBuf::from(user_path))
    }
}
