//! Tool registry, MCP/LSP wiring, and skill catalog.

use anycode_config::Config;

use anycode_core::prelude::*;
use anycode_security::SecurityLayer;
use anycode_tools::{
    build_registry_with_services, default_skill_roots, validate_default_registry,
    CompiledClaudePermissionRules, LspConnectionConfig, SkillCatalog, ToolServices,
};

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

use super::skills_registry;

pub struct ToolsSetup {
    pub tool_services: Arc<ToolServices>,
    pub tools: HashMap<ToolName, Box<dyn Tool>>,
    pub skill_catalog: Arc<SkillCatalog>,
    pub claude_rules: Arc<CompiledClaudePermissionRules>,
    pub expose_skill_on_explore_plan: bool,
}

pub async fn build_tools_setup(
    config: &Config,
    mcp_defer_gate: Option<Arc<Mutex<HashSet<String>>>>,
    security: &SecurityLayer,
    fw_policy: &anycode_security::SecurityPolicy,
    skill_project_root: Option<&Path>,
) -> anyhow::Result<ToolsSetup> {
    let mut merged_extra = config.skills.extra_dirs.clone();
    if let Some(ref ru) = config.skills.registry_url {
        let more = skills_registry::fetch_extra_skill_roots(ru).await;
        merged_extra.extend(more);
    }

    if config.skills.enabled {
        if let Some(home) = dirs::home_dir() {
            let skills_dest = home.join(".anycode/skills");
            match anycode_tools::ensure_office_starter_skills(&skills_dest) {
                Ok(installed) if !installed.is_empty() => {
                    tracing::info!(
                        target: "anycode_bootstrap",
                        count = installed.len(),
                        ids = ?installed.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
                        "installed default office starter skills"
                    );
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!(
                        target: "anycode_bootstrap",
                        error = %e,
                        "default office starter skills install skipped"
                    );
                }
            }
        }
    }

    let skill_catalog: Arc<SkillCatalog> = Arc::new(if config.skills.enabled {
        let mut roots = default_skill_roots(&merged_extra, dirs::home_dir().as_deref());
        if let Some(root) = skill_project_root {
            roots.push(root.join("skills"));
            roots.push(root.join(".anycode/skills"));
        }
        SkillCatalog::scan(
            &roots,
            config.skills.allowlist.as_deref(),
            config.skills.run_timeout_ms,
            config.skills.minimal_env,
        )
    } else {
        SkillCatalog::scan(
            &[],
            None,
            config.skills.run_timeout_ms,
            config.skills.minimal_env,
        )
    });

    let tool_services: Arc<ToolServices> = {
        let ts = if let Some(h) = dirs::home_dir() {
            let path = h.join(".anycode/tasks/orchestration.json");
            ToolServices::load_or_new_with_mcp_defer(
                path,
                mcp_defer_gate.clone(),
                skill_catalog.clone(),
            )
            .map_err(|e| anyhow::anyhow!("orchestration ToolServices: {e}"))?
        } else {
            ToolServices::new_ephemeral_with_skills(mcp_defer_gate.clone(), skill_catalog.clone())
        };
        #[cfg(feature = "tools-browser")]
        ts.set_browser_service(anycode_browser::BrowserService::shared());
        Arc::new(ts)
    };

    let claude_rules = CompiledClaudePermissionRules::compile(
        &config.security.mcp_tool_deny_rules,
        &config.security.always_allow_rules,
        &config.security.always_ask_rules,
    );

    #[cfg(feature = "tools-mcp")]
    {
        use super::browser_mcp;
        use super::mcp_env::{self, McpHttpTransport, McpServerEntry};
        use anycode_tools::{mcp_connected::McpConnected, mcp_rmcp_session::McpRmcpSession};
        let mut entries = mcp_env::mcp_server_entries_merged(&config.mcp.servers, true);
        if config.mcp.browser.enabled {
            let slug = browser_mcp::browser_mcp_slug().to_string();
            let already = entries.iter().any(|e| match e {
                McpServerEntry::Stdio { slug: s, .. } | McpServerEntry::Http { slug: s, .. } => {
                    s == &slug
                }
            });
            if !already {
                if let Some(root) = browser_mcp::resolve_browser_mcp_bundle_root() {
                    entries.push(McpServerEntry::Stdio {
                        slug,
                        command: browser_mcp::browser_mcp_stdio_command(&root),
                    });
                } else {
                    tracing::warn!(
                        target: "anycode_cli",
                        "mcp.browser.enabled but ANYCODE_BROWSER_MCP_ROOT bundle not found"
                    );
                }
            }
        }
        for entry in entries {
            match entry {
                McpServerEntry::Stdio { slug, command } => {
                    match anycode_tools::mcp_session::McpStdioSession::connect(&command, &slug)
                        .await
                    {
                        Ok(sess) => {
                            tool_services.attach_mcp_stdio(Arc::new(sess));
                            tracing::info!(target: "anycode_bootstrap", slug = %slug, "MCP stdio connected");
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "anycode_bootstrap",
                                slug = %slug,
                                error = %e,
                                "MCP stdio connect failed"
                            );
                        }
                    }
                }
                McpServerEntry::Http {
                    slug,
                    url,
                    transport,
                    bearer_token,
                    oauth_credentials_path,
                    headers,
                } => {
                    let connect = async {
                        match transport {
                            McpHttpTransport::LegacySse => {
                                if oauth_credentials_path.is_some() {
                                    Err(CoreError::LLMError(
                                        "MCP legacy SSE 暂不支持 oauth_credentials_path；请用 bearer_token 或改用 streamable-http"
                                            .into(),
                                    ))
                                } else {
                                    anycode_tools::mcp_legacy_sse_session::McpLegacySseSession::connect(
                                        &url,
                                        &slug,
                                        bearer_token.as_deref(),
                                        &headers,
                                    )
                                    .await
                                    .map(|s| Arc::new(s) as Arc<dyn McpConnected>)
                                }
                            }
                            McpHttpTransport::StreamableHttp => {
                                if let Some(ref cred_path) = oauth_credentials_path {
                                    McpRmcpSession::connect_streamable_http_oauth(
                                        &url, &slug, cred_path, &headers,
                                    )
                                    .await
                                    .map(|s| Arc::new(s) as Arc<dyn McpConnected>)
                                } else {
                                    McpRmcpSession::connect_streamable_http(
                                        &url,
                                        &slug,
                                        bearer_token.as_deref(),
                                        &headers,
                                    )
                                    .await
                                    .map(|s| Arc::new(s) as Arc<dyn McpConnected>)
                                }
                            }
                        }
                    };
                    match connect.await {
                        Ok(sess) => {
                            tool_services.attach_mcp_session(sess);
                            tracing::info!(
                                target: "anycode_bootstrap",
                                slug = %slug,
                                url = %url,
                                "MCP HTTP connected"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                target: "anycode_bootstrap",
                                slug = %slug,
                                url = %url,
                                error = %e,
                                "MCP HTTP connect failed"
                            );
                        }
                    }
                }
            }
        }
    }

    let lsp_cmd = if config.lsp.enabled {
        config.lsp.command.clone()
    } else {
        None
    };
    tool_services.set_lsp_connection_config(LspConnectionConfig {
        command: lsp_cmd,
        workspace_root: config.lsp.workspace_root.clone(),
        read_timeout: std::time::Duration::from_millis(config.lsp.read_timeout_ms),
    });

    let tools = build_registry_with_services(config.security.sandbox_mode, tool_services.clone());
    validate_default_registry(&tools)?;

    for name in tools.keys() {
        if name.starts_with("mcp__") {
            security.set_tool_policy(name, fw_policy.clone()).await;
        }
    }

    let expose_skill_on_explore_plan =
        config.skills.enabled && config.skills.expose_on_explore_plan;

    Ok(ToolsSetup {
        tool_services,
        tools,
        skill_catalog,
        claude_rules,
        expose_skill_on_explore_plan,
    })
}
