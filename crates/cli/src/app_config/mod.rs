//! 用户配置（`~/.anycode/config.json`）、运行时 `Config` 与 `model` / `config` 子命令逻辑。

mod config_wizard;
mod load;
mod model_interactive;
mod model_wizard;
mod onboard;
mod prompts;
mod schema;
mod user_config;

pub(crate) use schema::*;
pub(crate) use user_config::*;

pub(crate) use config_wizard::{
    disable_feature_flag, enable_feature_flag, run_config_wizard, set_default_runtime_mode,
};
pub(crate) use load::{
    apply_wechat_bridge_no_tool_approval, load_config_for_session, load_runtime_config,
    security_wants_interactive_approval_callback, LoadOpts,
};
pub(crate) use model_wizard::run_model_command;
pub(crate) use onboard::run_onboard_flow;

/// 无子命令时：`anycode model` 交互配置（OpenClaw 全目录 + 按任务路由）。
pub(crate) async fn run_model_interactive(
    config_file: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    model_interactive::run(config_file).await
}

/// `anycode setup` 场景：精简模型配置（仅 provider/model/key），不进入 routing 菜单。
pub(crate) async fn run_model_onboard_interactive(
    config_file: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    model_interactive::run_onboard(config_file).await
}

#[cfg(test)]
mod serde_config_tests {
    use super::*;

    #[test]
    fn auto_compact_threshold_prefers_absolute() {
        let mut s = SessionConfig::default();
        s.auto_compact_min_input_tokens = 50_000;
        s.context_window_tokens = 100_000;
        s.auto_compact_ratio = 0.5;
        assert_eq!(session_auto_compact_threshold(&s, 200_000), 50_000);
    }

    #[test]
    fn should_auto_compact_respects_zero_last_tokens() {
        let mut s = SessionConfig::default();
        s.context_window_auto = false;
        s.context_window_tokens = 128_000;
        assert!(!should_auto_compact_before_send(&s, "z.ai", "glm-5", 0));
        assert!(!should_auto_compact_before_send(
            &s, "z.ai", "glm-5", 100_000
        ));
        assert!(should_auto_compact_before_send(
            &s, "z.ai", "glm-5", 120_000
        ));
    }

    #[test]
    fn auto_resolved_window_claude_triggers_near_200k() {
        let s = SessionConfig::default();
        assert!(should_auto_compact_before_send(
            &s,
            "anthropic",
            "claude-3-5-sonnet-20241022",
            180_000
        ));
        assert!(!should_auto_compact_before_send(
            &s,
            "anthropic",
            "claude-3-5-sonnet-20241022",
            170_000
        ));
    }

    #[test]
    fn deserializes_legacy_json_without_security_or_routing() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.security.permission_mode, "default");
        assert!(c.security.require_approval);
        assert!(!c.security.sandbox_mode);
        assert!(c.routing.agents.is_empty());
        assert!(c.system_prompt_override.is_none());
        assert!(c.system_prompt_append.is_none());
        assert!(c.session.auto_compact);
        assert!(c.session.context_window_auto);
    }

    #[test]
    fn deserializes_session_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "session": {
                "auto_compact": false,
                "auto_compact_min_input_tokens": 90000,
                "context_window_tokens": 200000
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert!(!c.session.auto_compact);
        assert_eq!(c.session.auto_compact_min_input_tokens, 90_000);
        assert_eq!(c.session.context_window_tokens, 200_000);
    }

    #[test]
    fn deserializes_notifications_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "notifications": {
                "http_url": "http://127.0.0.1:9/hook",
                "http_headers": { "Authorization": "Bearer ${HOOKS_TOKEN}" },
                "shell_command": "cat",
                "max_body_bytes": 1024,
                "tool_deny_prefixes": ["mcp__"]
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        validate_notifications(&c.notifications).unwrap();
        assert!(c.notifications.is_configured());
        assert_eq!(
            c.notifications.http_url.as_deref(),
            Some("http://127.0.0.1:9/hook")
        );
        assert_eq!(c.notifications.max_body_bytes, 1024);
        assert_eq!(
            c.notifications.tool_deny_prefixes,
            vec!["mcp__".to_string()]
        );
    }

    #[test]
    fn notifications_validation_rejects_non_http_url_scheme() {
        let mut s = anycode_core::SessionNotificationSettings::default();
        s.http_url = Some("ftp://example.com/hook".to_string());
        s.max_body_bytes = 4096;
        let e = validate_notifications(&s).unwrap_err();
        let m = e.to_string();
        assert!(
            m.contains("notifications") && m.contains("http"),
            "unexpected message: {m}"
        );
    }

    #[test]
    fn notifications_validation_rejects_max_body_out_of_range() {
        let mut s = anycode_core::SessionNotificationSettings::default();
        s.max_body_bytes = 100;
        let e = validate_notifications(&s).unwrap_err();
        assert!(e.to_string().contains("max_body_bytes"), "{}", e);
    }

    #[test]
    fn deserializes_memory_and_zai_tool_flag() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "memory": {
                "backend": "hybrid",
                "path": ".anycode/mem-test",
                "auto_save": false
            },
            "zai_tool_choice_first_turn": true
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.memory.backend, "hybrid");
        assert_eq!(
            c.memory.path.as_ref().and_then(|p| p.to_str()),
            Some(".anycode/mem-test")
        );
        assert!(!c.memory.auto_save);
        assert!(c.zai_tool_choice_first_turn);
    }

    #[test]
    fn deserializes_memory_pipeline_embedding_local_fields() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "memory": {
                "backend": "pipeline",
                "pipeline": {
                    "embedding_provider": "local",
                    "embedding_local_model": "BGESmallZHV15",
                    "embedding_hf_endpoint": "https://hf-mirror.com"
                }
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.memory.backend, "pipeline");
        assert_eq!(
            c.memory.pipeline.embedding_provider.as_deref(),
            Some("local")
        );
        assert_eq!(
            c.memory.pipeline.embedding_local_model.as_deref(),
            Some("BGESmallZHV15")
        );
        assert_eq!(
            c.memory.pipeline.embedding_hf_endpoint.as_deref(),
            Some("https://hf-mirror.com")
        );
    }

    #[test]
    fn deserializes_security_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "security": {
                "permission_mode": "bypass",
                "require_approval": false,
                "sandbox_mode": true
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.security.permission_mode, "bypass");
        assert!(!c.security.require_approval);
        assert!(c.security.sandbox_mode);
        validate_permission_mode(&c.security.permission_mode).unwrap();
    }

    #[test]
    fn deserializes_mcp_tool_deny_rules() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "security": {
                "mcp_tool_deny_rules": ["mcp__slack", "Bash"]
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.security.mcp_tool_deny_rules, vec!["mcp__slack", "Bash"]);
    }

    #[test]
    fn deserializes_mcp_tool_deny_patterns() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "security": {
                "mcp_tool_deny_patterns": ["^mcp__secret__.*"]
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(c.security.mcp_tool_deny_patterns.len(), 1);
        assert_eq!(c.security.mcp_tool_deny_patterns[0], "^mcp__secret__.*");
    }

    #[test]
    fn permission_mode_invalid_is_rejected() {
        assert!(validate_permission_mode("typo").is_err());
        assert!(validate_permission_mode("").is_err());
    }

    #[test]
    fn llm_provider_anthropic_ok() {
        validate_llm_provider("anthropic").unwrap();
        validate_llm_provider("claude").unwrap();
    }

    #[test]
    fn llm_provider_openclaw_catalog_and_kebab() {
        validate_llm_provider("groq").unwrap();
        validate_llm_provider("fireworks").unwrap();
        validate_llm_provider("cloudflare-ai-gateway").unwrap();
        validate_llm_provider("amazon-bedrock").unwrap();
        validate_llm_provider("kimi").unwrap();
    }

    #[test]
    fn llm_provider_invalid_is_rejected() {
        assert!(validate_llm_provider("totally-unknown-vendor-xyz").is_err());
    }

    #[test]
    fn session_model_zai_must_be_catalog() {
        validate_session_model_override("z.ai", "glm-5").unwrap();
        validate_session_model_override("z.ai", "glm-5.1").unwrap();
        assert!(validate_session_model_override("z.ai", "not-a-catalog-model").is_err());
    }

    #[test]
    fn session_model_openrouter_allows_non_empty_id() {
        validate_session_model_override("openrouter", "anthropic/claude-3.5-sonnet").unwrap();
    }

    #[test]
    fn session_model_qualified_zai_suffix_must_be_catalog() {
        validate_session_model_override("openrouter", "z.ai/glm-5").unwrap();
        assert!(validate_session_model_override("openrouter", "z.ai/not-in-catalog").is_err());
    }

    #[test]
    fn deserializes_status_line_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "statusLine": {
                "command": "~/.anycode/statusline.sh",
                "timeout_ms": 4000,
                "padding": 3,
                "show_builtin": true
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert_eq!(
            c.status_line.command.as_deref(),
            Some("~/.anycode/statusline.sh")
        );
        assert_eq!(c.status_line.timeout_ms, Some(4000));
        assert_eq!(c.status_line.padding, Some(3));
        assert!(c.status_line.show_builtin);
    }

    #[test]
    fn status_line_runtime_trims_blank_command_to_none() {
        let f = StatusLineConfigFile {
            command: Some("  \n\t  ".to_string()),
            timeout_ms: None,
            padding: None,
            show_builtin: false,
        };
        let r: StatusLineRuntime = f.into();
        assert!(r.command.is_none());
        assert_eq!(r.timeout_ms, 5000);
    }

    #[test]
    fn deserializes_lsp_block() {
        let j = r#"{
            "provider":"z.ai",
            "plan":"coding",
            "api_key":"k",
            "base_url":null,
            "model":"glm-5",
            "temperature":0.7,
            "max_tokens":8192,
            "lsp": {
                "enabled": true,
                "command": "rust-analyzer",
                "workspace_root": "./myproj",
                "read_timeout_ms": 120000
            }
        }"#;
        let c: AnyCodeConfig = serde_json::from_str(j).unwrap();
        assert!(c.lsp.enabled);
        assert_eq!(c.lsp.command.as_deref(), Some("rust-analyzer"));
        assert_eq!(
            c.lsp.workspace_root.as_deref(),
            Some(std::path::Path::new("./myproj"))
        );
        assert_eq!(c.lsp.read_timeout_ms, Some(120_000));
    }
}
