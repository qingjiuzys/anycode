//! Open-ended verification nudge: block hollow "please recompile" completions without tool evidence.
//!
//! Does not register language-specific validators — only tracks whether the agent wrote files
//! and ran discover/search/run tools before claiming done.

use serde_json::Value;

const MAX_EVIDENCE_REPAIRS: u32 = 1;

const REPAIR_MESSAGE_ZH: &str = "你尚未用工具验证本次修改。请先：1) 读仓库 README/配置文件；2) 如不确定，用 WebSearch/WebFetch 查该技术栈的官方验证/编译方式；3) 用 Bash 实际执行并把输出作为证据；4) 验证通过后再声明完成。不要只让用户「重新编译」或「打开开发者工具看看」。";

const REPAIR_MESSAGE_EN: &str = "You have not verified this change with tools yet. Read repo docs/config, search official verify/build steps if needed, run them via Bash (or Browser when UI proof is required), and only then claim done. Do not ask the user to recompile or open an IDE as your only proof.";

/// Tracks file writes and verification actions across an entire task/turn session.
#[derive(Debug, Default, Clone)]
pub struct SessionVerificationState {
    pub wrote_files: bool,
    pub ran_verification: bool,
}

impl SessionVerificationState {
    pub fn note_tool(&mut self, tool_name: &str, tool_text: &str) {
        if is_write_tool(tool_name) {
            self.wrote_files = true;
        }
        if is_verification_tool(tool_name) && verification_tool_succeeded(tool_name, tool_text) {
            self.ran_verification = true;
        }
    }
}

pub fn is_write_tool(name: &str) -> bool {
    matches!(name, "FileWrite" | "Edit" | "NotebookEdit")
}

pub fn is_verification_tool(name: &str) -> bool {
    matches!(name, "Bash" | "WebSearch" | "WebFetch")
        || name.starts_with("Browser")
        || name == "PowerShell"
}

pub fn verification_tool_succeeded(tool_name: &str, tool_text: &str) -> bool {
    if tool_name == "Bash" || tool_name == "PowerShell" {
        return bash_exit_success(tool_text);
    }
    if tool_name == "WebSearch" || tool_name == "WebFetch" {
        let lower = tool_text.to_ascii_lowercase();
        return !lower.contains("error")
            && !lower.contains("failed")
            && !tool_text.trim().is_empty();
    }
    if tool_name.starts_with("Browser") {
        return !tool_text.trim().is_empty();
    }
    false
}

fn bash_exit_success(tool_text: &str) -> bool {
    if let Ok(v) = serde_json::from_str::<Value>(tool_text) {
        if let Some(code) = v.get("exit_code").and_then(|c| c.as_i64()) {
            return code == 0;
        }
    }
    let lower = tool_text.to_ascii_lowercase();
    if lower.contains("command failed") {
        return false;
    }
    if lower.contains("\"exit_code\":") {
        return lower.contains("\"exit_code\":0") || lower.contains("\"exit_code\": 0");
    }
    !lower.contains("exit_code=1") && !tool_text.trim().is_empty()
}

pub fn hollow_completion_phrase(text: &str) -> bool {
    let t = text.to_lowercase();
    let zh = text;
    [
        "重新编译",
        "开发者工具",
        "编译即可",
        "已全部修正",
        "全部问题已修复",
        "已修好",
        "已修复",
        "recompile",
        "re-compile",
        "open the ide",
        "open wechat",
        "developer tools",
        "try it yourself",
        "please compile",
        "should work now",
        "all fixed",
        "all issues fixed",
    ]
    .iter()
    .any(|p| t.contains(p) || zh.contains(p))
}

/// Returns repair message when agent wrote files, claims done without verification, and budget remains.
pub fn maybe_evidence_repair(
    state: &SessionVerificationState,
    assistant_text: &str,
    evidence_repairs_used: u32,
) -> Option<String> {
    if evidence_repairs_used >= MAX_EVIDENCE_REPAIRS {
        return None;
    }
    if !state.wrote_files {
        return None;
    }
    if state.ran_verification {
        return None;
    }
    let text = assistant_text.trim();
    if text.is_empty() {
        return None;
    }
    if !hollow_completion_phrase(text) {
        return None;
    }
    Some(if text.chars().any(|c| c as u32 >= 0x4E00) {
        REPAIR_MESSAGE_ZH.to_string()
    } else {
        REPAIR_MESSAGE_EN.to_string()
    })
}

/// Parse a successful Bash command for verify_recipe memory.
pub fn try_verify_recipe_from_bash(command: &str, tool_text: &str, cwd: &str) -> Option<String> {
    if !bash_exit_success(tool_text) {
        return None;
    }
    let cmd = command.trim();
    if cmd.is_empty() || cmd.len() > 500 {
        return None;
    }
    let stack = infer_stack_hint(cmd, cwd);
    Some(format!("verify_recipe: {stack} → {cmd} @ {cwd}"))
}

fn infer_stack_hint(command: &str, cwd: &str) -> String {
    let c = command.to_ascii_lowercase();
    let p = cwd.to_ascii_lowercase();
    if c.contains("docker compose") || c.contains("docker-compose") {
        return "docker-compose".into();
    }
    if p.contains("miniprogram") || c.contains("cli preview") || c.contains("miniprogram-ci") {
        return "wechat-miniprogram".into();
    }
    if c.contains("cargo ") {
        return "rust".into();
    }
    if c.contains("npm ") || c.contains("pnpm ") || c.contains("yarn ") {
        return "node".into();
    }
    if c.contains("pytest") || c.contains("python ") {
        return "python".into();
    }
    "shell".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_hollow_phrases() {
        assert!(hollow_completion_phrase("已全部修正。重新编译即可。"));
        assert!(!hollow_completion_phrase("cargo test passed with 12 tests"));
    }

    #[test]
    fn nudge_when_wrote_without_verify() {
        let mut s = SessionVerificationState::default();
        s.wrote_files = true;
        assert!(maybe_evidence_repair(&s, "重新编译即可", 0).is_some());
        assert!(maybe_evidence_repair(&s, "重新编译即可", 1).is_none());
    }

    #[test]
    fn no_nudge_when_verified() {
        let mut s = SessionVerificationState::default();
        s.wrote_files = true;
        s.ran_verification = true;
        assert!(maybe_evidence_repair(&s, "重新编译即可", 0).is_none());
    }

    #[test]
    fn bash_json_exit_zero_counts() {
        assert!(verification_tool_succeeded(
            "Bash",
            r#"{"exit_code":0,"stdout":"ok"}"#
        ));
        assert!(!verification_tool_succeeded(
            "Bash",
            r#"{"exit_code":1,"stderr":"fail"}"#
        ));
    }

    #[test]
    fn verify_recipe_from_docker() {
        let r = try_verify_recipe_from_bash(
            "docker compose config",
            r#"{"exit_code":0}"#,
            "/tmp/pindou",
        );
        assert!(r.unwrap().contains("docker compose config"));
    }
}
