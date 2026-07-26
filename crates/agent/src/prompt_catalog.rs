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

const LOCALE_ZH_REPLY_LANGUAGE: &str = include_str!("../prompts/locale/zh/reply_language.md");
const LOCALE_EN_REPLY_LANGUAGE: &str = include_str!("../prompts/locale/en/reply_language.md");
const LOCALE_ZH_EPHEMERAL: &str = include_str!("../prompts/locale/zh/ephemeral_reminder.md");
const LOCALE_EN_EPHEMERAL: &str = include_str!("../prompts/locale/en/ephemeral_reminder.md");

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
        _ => "",
    }
}

fn locale_file(tag: &str, name: &str) -> Option<&'static str> {
    match (tag, name) {
        ("zh", "reply_language") => Some(LOCALE_ZH_REPLY_LANGUAGE),
        ("en", "reply_language") => Some(LOCALE_EN_REPLY_LANGUAGE),
        ("zh", "ephemeral_reminder") => Some(LOCALE_ZH_EPHEMERAL),
        ("en", "ephemeral_reminder") => Some(LOCALE_EN_EPHEMERAL),
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
    parts.push(core("media_generation").trim().to_string());
    parts.push(core("plan_progress").trim().to_string());
    if include_browser {
        parts.push(core("browser").trim().to_string());
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn core_and_locale_files_are_nonempty() {
        assert!(CORE_TONE.contains("# Tone"));
        assert!(CORE_TONE.contains("coding agent"));
        assert!(CORE_AGENT_LOOP.contains("{tools}"));
        assert!(LOCALE_ZH_REPLY_LANGUAGE.contains("中文"));
        assert!(LOCALE_EN_REPLY_LANGUAGE.contains("English"));
        assert!(!LOCALE_ZH_EPHEMERAL.trim().is_empty());
        assert!(!LOCALE_EN_EPHEMERAL.trim().is_empty());
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
