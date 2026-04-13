//! LSP `initialize` 的 `rootUri` 序列化（不依赖 `tools-lsp` 子进程逻辑，便于默认 CI 单测）。

use serde_json::{json, Value};
use std::path::Path;

/// `initialize` 的 `rootUri`：绝对路径且可被 `Url::from_file_path` 接受时为 `file://` 字符串，否则 `null`。
///
/// 实现放在此模块以便默认 **`cargo test`** 可跑 URI 单测；实际调用在 **`lsp_stdio`**（`--features tools-lsp`）。
#[cfg_attr(not(feature = "tools-lsp"), allow(dead_code))]
pub(crate) fn lsp_root_uri_json(workspace_root: Option<&Path>) -> Value {
    workspace_root
        .and_then(|p| url::Url::from_file_path(p).ok())
        .map(|u| json!(u.as_str()))
        .unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::lsp_root_uri_json;
    use serde_json::json;
    use std::path::Path;

    #[test]
    fn root_uri_null_when_none() {
        assert_eq!(lsp_root_uri_json(None), serde_json::Value::Null);
    }

    #[test]
    fn root_uri_file_scheme_for_canonical_dir() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let abs = std::fs::canonicalize(tmp.path()).expect("canonicalize");
        let v = lsp_root_uri_json(Some(abs.as_path()));
        let s = v.as_str().expect("string uri");
        assert!(s.starts_with("file://"), "expected file URI, got {s:?}");
    }

    #[test]
    fn root_uri_null_for_relative_path() {
        let v = lsp_root_uri_json(Some(Path::new("relative/nope")));
        assert_eq!(v, json!(null));
    }
}
