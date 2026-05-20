//! Runtime tool policy profiles for headless, CI, and channel execution surfaces.
//!
//! Cron jobs use [`super::catalog::cron_tool_profile_filters`] directly via `RunTaskOptions`;
//! this module adds config/env-driven profiles for non-cron task entrypoints.

use super::catalog::cron_tool_profile_filters;

/// Execution context for resolving additive tool deny lists (merged into `TaskContext`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolExecutionSurface {
    /// Stream REPL / interactive TTY session (no automatic profile unless env/config override).
    Interactive,
    /// Headless single-task runs (`anycode run`, scheduler without per-job profile).
    Headless,
    /// CI / automation hosts (`CI=true`, `GITHUB_ACTIONS=true`, or explicit surface).
    Ci,
    /// Channel bridges (WeChat, Telegram, Discord).
    Channel,
}

/// Per-surface named profiles (`default`, `read_only`, `observability`, `allowlist`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ToolPolicyProfiles {
    pub headless: Option<String>,
    pub ci: Option<String>,
    pub channel: Option<String>,
}

pub struct RuntimeToolPolicyInput<'a> {
    pub surface: ToolExecutionSurface,
    pub profiles: &'a ToolPolicyProfiles,
    /// Cron/job profile (highest priority when set).
    pub explicit_profile: Option<&'a str>,
    pub explicit_allowlist: Option<&'a [String]>,
    /// Config `runtime.tool_deny_names` / `tool_deny_prefixes` (additive).
    pub extra_deny_names: &'a [String],
    pub extra_deny_prefixes: &'a [String],
}

const DEFAULT_CI_PROFILE: &str = "read_only";
const DEFAULT_CHANNEL_PROFILE: &str = "observability";

/// True when common CI env vars indicate an automation host.
pub fn detect_ci_environment() -> bool {
    matches!(
        std::env::var("CI").ok().as_deref(),
        Some("1") | Some("true") | Some("TRUE") | Some("yes") | Some("YES")
    ) || std::env::var("GITHUB_ACTIONS").ok().as_deref() == Some("true")
}

fn merge_csv_env(var: &str) -> Vec<String> {
    std::env::var(var)
        .ok()
        .map(|s| {
            s.split(',')
                .map(str::trim)
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn push_unique(list: &mut Vec<String>, value: String) {
    if !value.is_empty() && !list.contains(&value) {
        list.push(value);
    }
}

fn surface_profile(surface: ToolExecutionSurface, profiles: &ToolPolicyProfiles) -> Option<String> {
    let configured = match surface {
        ToolExecutionSurface::Headless => profiles.headless.as_ref(),
        ToolExecutionSurface::Ci => profiles.ci.as_ref(),
        ToolExecutionSurface::Channel => profiles.channel.as_ref(),
        ToolExecutionSurface::Interactive => return None,
    };
    if let Some(p) = configured {
        let t = p.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    match surface {
        ToolExecutionSurface::Ci => Some(DEFAULT_CI_PROFILE.to_string()),
        ToolExecutionSurface::Channel => Some(DEFAULT_CHANNEL_PROFILE.to_string()),
        _ => None,
    }
}

/// Resolve `(tool_deny_names, tool_deny_prefixes)` for a task from profile + env + config extras.
pub fn resolve_runtime_tool_filters(
    input: RuntimeToolPolicyInput<'_>,
) -> (Vec<String>, Vec<String>) {
    let profile_name = input
        .explicit_profile
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            std::env::var("ANYCODE_TOOL_PROFILE")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
        })
        .or_else(|| surface_profile(input.surface, input.profiles));

    let (mut names, mut prefixes) =
        cron_tool_profile_filters(profile_name.as_deref(), input.explicit_allowlist);

    for n in input.extra_deny_names {
        push_unique(&mut names, n.clone());
    }
    for p in input.extra_deny_prefixes {
        push_unique(&mut prefixes, p.clone());
    }
    for n in merge_csv_env("ANYCODE_TOOL_DENY") {
        push_unique(&mut names, n);
    }
    for p in merge_csv_env("ANYCODE_TOOL_DENY_PREFIXES") {
        push_unique(&mut prefixes, p);
    }

    (names, prefixes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clear_tool_policy_env() {
        std::env::remove_var("ANYCODE_TOOL_PROFILE");
        std::env::remove_var("ANYCODE_TOOL_DENY");
        std::env::remove_var("ANYCODE_TOOL_DENY_PREFIXES");
    }

    #[test]
    fn ci_surface_defaults_to_read_only() {
        clear_tool_policy_env();
        let profiles = ToolPolicyProfiles::default();
        let (names, prefixes) = resolve_runtime_tool_filters(RuntimeToolPolicyInput {
            surface: ToolExecutionSurface::Ci,
            profiles: &profiles,
            explicit_profile: None,
            explicit_allowlist: None,
            extra_deny_names: &[],
            extra_deny_prefixes: &[],
        });
        assert!(names.iter().any(|n| n == "Bash"));
        assert!(prefixes.iter().any(|p| p == "mcp__"));
    }

    #[test]
    fn channel_surface_defaults_to_observability() {
        clear_tool_policy_env();
        let profiles = ToolPolicyProfiles::default();
        let (names, _) = resolve_runtime_tool_filters(RuntimeToolPolicyInput {
            surface: ToolExecutionSurface::Channel,
            profiles: &profiles,
            explicit_profile: None,
            explicit_allowlist: None,
            extra_deny_names: &[],
            extra_deny_prefixes: &[],
        });
        assert!(!names.iter().any(|n| n == "TaskList"));
        assert!(names.iter().any(|n| n == "Bash"));
    }

    #[test]
    fn explicit_profile_overrides_surface_default() {
        clear_tool_policy_env();
        let profiles = ToolPolicyProfiles::default();
        let (names, _) = resolve_runtime_tool_filters(RuntimeToolPolicyInput {
            surface: ToolExecutionSurface::Channel,
            profiles: &profiles,
            explicit_profile: Some("default"),
            explicit_allowlist: None,
            extra_deny_names: &[],
            extra_deny_prefixes: &[],
        });
        assert!(names.is_empty());
    }

    #[test]
    fn extra_deny_names_merged_on_top_of_profile() {
        clear_tool_policy_env();
        let profiles = ToolPolicyProfiles::default();
        let extras = vec!["Glob".to_string(), "FileRead".to_string()];
        let (names, _) = resolve_runtime_tool_filters(RuntimeToolPolicyInput {
            surface: ToolExecutionSurface::Interactive,
            profiles: &profiles,
            explicit_profile: None,
            explicit_allowlist: None,
            extra_deny_names: &extras,
            extra_deny_prefixes: &[],
        });
        assert!(names.contains(&"Glob".to_string()));
        assert!(names.contains(&"FileRead".to_string()));
    }

    #[test]
    fn config_profile_overrides_builtin_default() {
        clear_tool_policy_env();
        let profiles = ToolPolicyProfiles {
            channel: Some("read_only".into()),
            ..Default::default()
        };
        let (names, _) = resolve_runtime_tool_filters(RuntimeToolPolicyInput {
            surface: ToolExecutionSurface::Channel,
            profiles: &profiles,
            explicit_profile: None,
            explicit_allowlist: None,
            extra_deny_names: &[],
            extra_deny_prefixes: &[],
        });
        assert!(names.iter().any(|n| n == "Bash"));
        assert!(!names.iter().any(|n| n == "TaskList"));
    }
}
