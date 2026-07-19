//! Resolve dashboard web-chat agent ids (UI "Auto" → config default or general-purpose).

pub const DEFAULT_WEB_CHAT_AGENT: &str = "general-purpose";

/// Empty / whitespace agent → `agents.defaults.run` from config, else [`DEFAULT_WEB_CHAT_AGENT`].
pub fn resolve_web_chat_agent(agent: Option<&str>) -> String {
    if let Some(a) = agent.map(str::trim).filter(|s| !s.is_empty()) {
        return a.to_string();
    }
    if let Ok((_, cfg)) = crate::config_patch::read_config_root() {
        if let Some(run) = cfg
            .get("agents")
            .and_then(|a| a.get("defaults"))
            .and_then(|d| d.get("run"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return run.to_string();
        }
    }
    DEFAULT_WEB_CHAT_AGENT.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_agent_is_preserved() {
        assert_eq!(resolve_web_chat_agent(Some("code")), "code");
        assert_eq!(resolve_web_chat_agent(Some("  plan  ")), "plan");
    }

    #[test]
    fn empty_agent_falls_back_to_general_purpose() {
        assert_eq!(resolve_web_chat_agent(None), DEFAULT_WEB_CHAT_AGENT);
        assert_eq!(resolve_web_chat_agent(Some("")), DEFAULT_WEB_CHAT_AGENT);
        assert_eq!(resolve_web_chat_agent(Some("   ")), DEFAULT_WEB_CHAT_AGENT);
    }
}
