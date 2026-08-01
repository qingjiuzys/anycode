//! Compile-time catalog of agent system-prompt markdown under `crates/agent/prompts/`.
//!
//! See [`prompts/STRATEGY.md`](../../prompts/STRATEGY.md) for the zh/en layering policy.

use std::collections::HashMap;

const CORE_TONE: &str = include_str!("../prompts/core/tone.md");
const CORE_ENVIRONMENT: &str = include_str!("../prompts/core/environment.md");
const CORE_AGENT_LOOP: &str = include_str!("../prompts/core/agent_loop.md");
const CORE_USER_CLARIFICATION: &str = include_str!("../prompts/core/user_clarification.md");
const CORE_MEDIA_GENERATION: &str = include_str!("../prompts/core/media_generation.md");
const CORE_PLAN_PROGRESS: &str = include_str!("../prompts/core/plan_progress.md");
const CORE_BROWSER: &str = include_str!("../prompts/core/browser.md");
const CORE_TOOL_PREFERENCES: &str = include_str!("../prompts/core/tool_preferences.md");
const CORE_GIT_WORKFLOW: &str = include_str!("../prompts/core/git_workflow.md");
const CORE_MCP_INTEGRATION: &str = include_str!("../prompts/core/mcp_integration.md");
const CORE_HOOKS_CONFIGURATION: &str = include_str!("../prompts/core/hooks_configuration.md");
const CORE_OUTPUT_FORMAT: &str = include_str!("../prompts/core/output_format.md");

const LOCALE_ZH_REPLY_LANGUAGE: &str = include_str!("../prompts/locale/zh/reply_language.md");
const LOCALE_EN_REPLY_LANGUAGE: &str = include_str!("../prompts/locale/en/reply_language.md");
const LOCALE_ZH_EPHEMERAL: &str = include_str!("../prompts/locale/zh/ephemeral_reminder.md");
const LOCALE_EN_EPHEMERAL: &str = include_str!("../prompts/locale/en/ephemeral_reminder.md");
const LOCALE_ZH_OUTPUT_FORMAT: &str = include_str!("../prompts/locale/zh/output_format.md");
const LOCALE_EN_OUTPUT_FORMAT: &str = include_str!("../prompts/locale/en/output_format.md");

/// Normalize raw lang (`zh-CN`, `en`, …) to a catalog tag, or `None` if unsupported.
#[must_use]
pub(crate) fn resolve_locale_tag(raw: &str) -> Option<&'static str> {
    let lang = raw.trim().to_lowercase();
    if lang.starts_with("zh") {
        Some("zh")
    } else if lang.starts_with("en") {
        Some("en")
    } else {
        None
    }
}

/// Active reply-language tag from task-local context or `ANYCODE_REPLY_LANG`.
#[must_use]
pub(crate) fn active_locale_tag() -> Option<&'static str> {
    let raw = anycode_core::current_reply_language()
        .or_else(|| std::env::var("ANYCODE_REPLY_LANG").ok())?;
    resolve_locale_tag(&raw)
}

fn fill_template(template: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out.trim().to_string()
}

fn core(name: &str) -> &'static str {
    match name {
        "tone" => CORE_TONE,
        "environment" => CORE_ENVIRONMENT,
        "agent_loop" => CORE_AGENT_LOOP,
        "user_clarification" => CORE_USER_CLARIFICATION,
        "media_generation" => CORE_MEDIA_GENERATION,
        "plan_progress" => CORE_PLAN_PROGRESS,
        "browser" => CORE_BROWSER,
        "tool_preferences" => CORE_TOOL_PREFERENCES,
        "git_workflow" => CORE_GIT_WORKFLOW,
        "mcp_integration" => CORE_MCP_INTEGRATION,
        "hooks_configuration" => CORE_HOOKS_CONFIGURATION,
        "output_format" => CORE_OUTPUT_FORMAT,
        _ => "",
    }
}

fn locale_file(tag: &str, name: &str) -> Option<&'static str> {
    match (tag, name) {
        ("zh", "reply_language") => Some(LOCALE_ZH_REPLY_LANGUAGE),
        ("en", "reply_language") => Some(LOCALE_EN_REPLY_LANGUAGE),
        ("zh", "ephemeral_reminder") => Some(LOCALE_ZH_EPHEMERAL),
        ("en", "ephemeral_reminder") => Some(LOCALE_EN_EPHEMERAL),
        ("zh", "output_format") => Some(LOCALE_ZH_OUTPUT_FORMAT),
        ("en", "output_format") => Some(LOCALE_EN_OUTPUT_FORMAT),
        _ => None,
    }
}

/// `# Reply language` section for the active locale, if any.
#[must_use]
pub(crate) fn reply_language_section() -> Option<String> {
    let tag = active_locale_tag()?;
    let body = locale_file(tag, "reply_language")?;
    Some(body.trim().to_string())
}

/// Per-turn ephemeral reminder for the active locale, if any.
#[must_use]
pub(crate) fn ephemeral_reminder_text() -> Option<String> {
    let tag = active_locale_tag()?;
    let body = locale_file(tag, "ephemeral_reminder")?;
    Some(body.trim().to_string())
}

/// Build the default system-prompt stack sections (excluding Custom Agent Instructions).
#[must_use]
pub(crate) fn default_stack_sections(
    cwd: &str,
    tools: &[String],
    include_browser: bool,
) -> Vec<String> {
    let date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let os = std::env::consts::OS.to_string();
    let tools_joined = tools.join(", ");
    let include_media = tools
        .iter()
        .any(|t| t == "GenerateImage" || t == "GenerateVideo");
    let include_plan = tools.iter().any(|t| t == "PlanWrite");
    let include_mcp = tools
        .iter()
        .any(|t| t.starts_with("mcp__") || t == "ListMcpResourcesTool");

    let mut env_vars = HashMap::new();
    env_vars.insert("cwd", cwd.to_string());
    env_vars.insert("os", os);
    env_vars.insert("date", date);

    let mut loop_vars = HashMap::new();
    loop_vars.insert("tools", tools_joined);

    let mut parts = Vec::new();
    if let Some(lang) = reply_language_section() {
        parts.push(lang);
    }
    parts.push(core("tone").trim().to_string());
    parts.push(fill_template(core("environment"), &env_vars));
    parts.push(fill_template(core("agent_loop"), &loop_vars));
    parts.push(core("user_clarification").trim().to_string());
    if include_media {
        parts.push(core("media_generation").trim().to_string());
    }
    if include_plan {
        parts.push(core("plan_progress").trim().to_string());
    }
    if include_browser {
        parts.push(core("browser").trim().to_string());
    }
    parts.push(core("tool_preferences").trim().to_string());
    parts.push(core("git_workflow").trim().to_string());
    if include_mcp {
        parts.push(core("mcp_integration").trim().to_string());
    }
    parts.push(core("hooks_configuration").trim().to_string());
    if let Some(out) = locale_output_format_section() {
        parts.push(out);
    } else {
        parts.push(core("output_format").trim().to_string());
    }
    parts
}

/// `# Output format` section for the active locale (falls back to core English).
#[must_use]
pub(crate) fn locale_output_format_section() -> Option<String> {
    let tag = active_locale_tag()?;
    let body = locale_file(tag, "output_format")?;
    Some(body.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_and_locale_files_are_nonempty() {
        assert!(CORE_TONE.contains("# Tone"));
        assert!(CORE_TONE.contains("coding agent"));
        assert!(CORE_TONE.contains("never invent tool output"));
        assert!(CORE_AGENT_LOOP.contains("{tools}"));
        assert!(LOCALE_ZH_REPLY_LANGUAGE.contains("中文"));
        assert!(LOCALE_EN_REPLY_LANGUAGE.contains("English"));
        assert!(!LOCALE_ZH_EPHEMERAL.trim().is_empty());
        assert!(!LOCALE_EN_EPHEMERAL.trim().is_empty());
    }

    #[test]
    fn default_stack_omits_media_and_plan_without_tools() {
        let parts = default_stack_sections("/tmp", &["Bash".into()], false);
        let joined = parts.join("\n");
        assert!(!joined.contains("# Media generation"));
        assert!(!joined.contains("# Plan progress"));
    }

    #[test]
    fn default_stack_includes_media_and_plan_when_tools_present() {
        let parts = default_stack_sections(
            "/tmp",
            &["Bash".into(), "GenerateVideo".into(), "PlanWrite".into()],
            false,
        );
        let joined = parts.join("\n");
        assert!(joined.contains("# Media generation"));
        assert!(joined.contains("# Plan progress"));
    }

    #[test]
    fn resolve_locale_tag_zh_en() {
        assert_eq!(resolve_locale_tag("zh-CN"), Some("zh"));
        assert_eq!(resolve_locale_tag("en"), Some("en"));
        assert_eq!(resolve_locale_tag("ja"), None);
    }

    #[test]
    fn fill_template_replaces_placeholders() {
        let mut vars = HashMap::new();
        vars.insert("cwd", "/tmp".into());
        vars.insert("os", "macos".into());
        vars.insert("date", "2026-07-15".into());
        let out = fill_template(CORE_ENVIRONMENT, &vars);
        assert!(out.contains("/tmp"));
        assert!(out.contains("macos"));
        assert!(!out.contains("{cwd}"));
    }
}
