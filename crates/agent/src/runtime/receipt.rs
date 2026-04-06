//! Summary 阶段使用的 artifacts 摘要文本。

use anycode_core::Artifact;

pub(super) struct ReceiptGenerator;

impl ReceiptGenerator {
    pub(super) fn artifacts_brief(artifacts: &[Artifact]) -> String {
        if artifacts.is_empty() {
            return "（无）".to_string();
        }
        let mut lines: Vec<String> = vec![];
        for (i, a) in artifacts.iter().enumerate() {
            if let Some(p) = &a.path {
                lines.push(format!("{}. {}: {}", i + 1, a.name, p));
            } else if let Some(cmd) = a.metadata.get("command").and_then(|v| v.as_str()) {
                let exit_code = a
                    .metadata
                    .get("exit_code")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                lines.push(format!(
                    "{}. {}: command={} exit_code={}",
                    i + 1,
                    a.name,
                    cmd,
                    exit_code
                ));
            } else {
                lines.push(format!("{}. {}", i + 1, a.name));
            }
        }
        lines.join("\n")
    }
}
