//! Config schema/defaults/helpers extracted from `app_config.rs` to keep orchestration logic smaller.

use super::{is_anthropic_family_provider, is_known_zai_model, is_zai_family_provider};
use crate::i18n::{tr, tr_args};
use anycode_agent::{CompactPolicy, RuntimePromptConfig};
use anycode_core::{FeatureFlag, FeatureRegistry, ModelRouteProfile, RuntimeMode};
use anycode_llm::{
    is_known_provider_id, normalize_provider_id, resolve_chat_model_ref,
    resolve_context_window_tokens, zai_model_catalog_entries, ChatModelResolutionReason,
    ZAI_MODEL_CATALOG,
};
use anyhow::Context;
use fluent_bundle::FluentArgs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 配置结构
#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) llm: LLMConfig,
    pub(crate) memory: MemoryConfig,
    pub(crate) security: SecurityConfig,
    pub(crate) routing: RoutingConfig,
    pub(crate) runtime: RuntimeSettings,
    pub(crate) prompt: RuntimePromptConfig,
    pub(crate) skills: SkillsConfig,
    /// TUI 会话：自动压缩阈值等（`config.json` 的 `session` 段）。
    pub(crate) session: SessionConfig,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeSettings {
    pub(crate) default_mode: RuntimeMode,
    pub(crate) features: FeatureRegistry,
    pub(crate) model_routes: ModelRouteProfile,
    /// 当前工作目录在 `~/.anycode/workspace/projects/index.json` 中匹配到的项目标签（仅内存叠加，不写回全局配置）。
    pub(crate) workspace_project_label: Option<String>,
    /// 同上：项目级通道 profile 提示（如 `web` / `wechat`）。
    pub(crate) workspace_channel_profile: Option<String>,
}

/// 运行时 `session` 段（与 `SessionConfigFile` 对应）。
#[derive(Debug, Clone)]
pub(crate) struct SessionConfig {
    /// 在发送新用户消息前，若上一轮 LLM 报告的 input tokens 达到阈值则先压缩会话。
    pub(crate) auto_compact: bool,
    /// 绝对阈值（input tokens）；>0 时优先于 `auto_compact_ratio × 有效窗口`。
    pub(crate) auto_compact_min_input_tokens: u32,
    /// 与有效上下文窗口相乘得到阈值（默认 0.88）。
    pub(crate) auto_compact_ratio: f32,
    /// 为 `true` 时根据当前 `provider` + `model` 自动推断窗口（[`resolve_context_window_tokens`]）。
    pub(crate) context_window_auto: bool,
    /// `context_window_auto == false` 时用于比例阈值的手动窗口（tokens）。
    pub(crate) context_window_tokens: u32,
}

impl From<SessionConfigFile> for SessionConfig {
    fn from(f: SessionConfigFile) -> Self {
        Self {
            auto_compact: f.auto_compact,
            auto_compact_min_input_tokens: f.auto_compact_min_input_tokens,
            auto_compact_ratio: f.auto_compact_ratio,
            context_window_auto: f.context_window_auto,
            context_window_tokens: f.context_window_tokens,
        }
    }
}

impl Default for SessionConfig {
    fn default() -> Self {
        SessionConfigFile::default().into()
    }
}

/// 有效上下文窗口（tokens）：自动推断或手动配置。
pub(crate) fn effective_session_context_window_tokens(
    session: &SessionConfig,
    provider_raw: &str,
    model_id: &str,
) -> u32 {
    if session.context_window_auto {
        let norm = normalize_provider_id(provider_raw.trim());
        resolve_context_window_tokens(&norm, model_id.trim())
    } else {
        session.context_window_tokens
    }
}

/// 自动压缩触发阈值（input tokens）。`effective_context_window` 由 [`effective_session_context_window_tokens`] 得到。
pub(crate) fn session_auto_compact_threshold(
    cfg: &SessionConfig,
    effective_context_window: u32,
) -> u32 {
    let policy = CompactPolicy {
        trigger_ratio: cfg.auto_compact_ratio.clamp(0.01, 1.0),
        hard_token_threshold: cfg.auto_compact_min_input_tokens,
        suppress_follow_up_questions: true,
    };
    if policy.hard_token_threshold > 0 {
        policy.hard_token_threshold
    } else {
        let t = (effective_context_window as f32) * policy.trigger_ratio;
        if t >= u32::MAX as f32 {
            u32::MAX
        } else {
            t as u32
        }
    }
}

/// TUI：在追加用户消息并发起 turn 之前，是否应先跑一次会话压缩。
pub(crate) fn should_auto_compact_before_send(
    cfg: &SessionConfig,
    provider_raw: &str,
    model_id: &str,
    last_reported_max_input_tokens: u32,
) -> bool {
    if !cfg.auto_compact {
        return false;
    }
    if last_reported_max_input_tokens == 0 {
        return false;
    }
    let win = effective_session_context_window_tokens(cfg, provider_raw, model_id);
    let th = session_auto_compact_threshold(cfg, win);
    th > 0 && last_reported_max_input_tokens >= th
}

#[derive(Debug, Clone)]
pub(crate) struct LLMConfig {
    pub(crate) provider: String,
    pub(crate) plan: String,
    pub(crate) model: String,
    pub(crate) api_key: String,
    pub(crate) base_url: Option<String>,
    pub(crate) temperature: f32,
    pub(crate) max_tokens: u32,
    /// 额外厂商密钥（如全局为 z.ai 时在此存 `anthropic` key，供 routing 混用）。
    pub(crate) provider_credentials: HashMap<String, String>,
    /// z.ai / OpenAI 兼容栈：首轮 agent 请求在带 tools 时使用 `tool_choice: required`（与 `ANYCODE_ZAI_TOOL_CHOICE_FIRST_TURN` 等价；环境变量优先）。
    pub(crate) zai_tool_choice_first_turn: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct MemoryConfig {
    pub(crate) path: PathBuf,
    pub(crate) auto_save: bool,
    /// `noop` | `none` | `off` | `file` | `hybrid`（运行时小写归一）
    pub(crate) backend: String,
}

/// `config.json` 中的 `memory` 段（serde）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct MemoryConfigFile {
    /// `noop`（或 `none`/`off`）| `file` | `hybrid`；默认持久化 `file`。
    #[serde(default = "default_memory_backend_kind")]
    pub(crate) backend: String,
    /// 记忆根目录。默认 `$HOME/.anycode/memory`；**相对路径相对于 `$HOME`**。
    #[serde(default)]
    pub(crate) path: Option<PathBuf>,
    #[serde(default = "default_memory_auto_save_file")]
    pub(crate) auto_save: bool,
}

fn default_memory_backend_kind() -> String {
    "file".to_string()
}

fn default_memory_auto_save_file() -> bool {
    true
}

impl Default for MemoryConfigFile {
    fn default() -> Self {
        Self {
            backend: default_memory_backend_kind(),
            path: None,
            auto_save: default_memory_auto_save_file(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SecurityConfig {
    pub(crate) permission_mode: String,
    pub(crate) require_approval: bool,
    pub(crate) sandbox_mode: bool,
    /// 在交给模型前从工具列表中剔除名称匹配任一正则的项（常用于 `mcp__.*` 等）。
    pub(crate) mcp_tool_deny_patterns: Vec<String>,
    /// Claude `alwaysDeny` 式 blanket 串：`mcp__Server` 或 `mcp__Server__*` 整服屏蔽；与 `permissions.ts` `toolMatchesRule` 对齐。
    pub(crate) mcp_tool_deny_rules: Vec<String>,
    /// Claude `alwaysAllow`：blanket 或 `Tool(content)`；content 级在执行前求值，可覆盖 deny。
    pub(crate) always_allow_rules: Vec<String>,
    /// Claude `alwaysAsk`：命中后需交互确认（无回调时拒绝）。
    pub(crate) always_ask_rules: Vec<String>,
    /// 首轮从 LLM 工具列表隐藏全部 `mcp__*`，直至 `ToolSearch` 登记（与 Claude defer MCP 对齐）。
    pub(crate) defer_mcp_tools: bool,
    /// `-I` / `ANYCODE_IGNORE_APPROVAL`：本进程不注册交互式审批回调（不写入配置文件）。
    pub(crate) session_skip_interactive_approval: bool,
}

// ============================================================================
// anyCode 用户级配置（~/.anycode/config.json）
// ============================================================================

fn default_session_auto_compact() -> bool {
    true
}

fn default_auto_compact_ratio() -> f32 {
    0.88
}

fn default_context_window_tokens() -> u32 {
    128_000
}

fn default_context_window_auto() -> bool {
    true
}

/// `config.json` 的 `session` 段（serde）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SessionConfigFile {
    /// 发送新用户消息前是否可按阈值自动压缩会话。
    #[serde(default = "default_session_auto_compact")]
    pub(crate) auto_compact: bool,
    /// 绝对阈值（input tokens）；>0 时优先于比例阈值。
    #[serde(default)]
    pub(crate) auto_compact_min_input_tokens: u32,
    #[serde(default = "default_auto_compact_ratio")]
    pub(crate) auto_compact_ratio: f32,
    /// 为 `true` 时根据 `provider` + `model` 自动推断上下文窗口（见 anycode_llm）。
    #[serde(default = "default_context_window_auto")]
    pub(crate) context_window_auto: bool,
    /// `context_window_auto == false` 时使用的手动窗口大小（tokens）。
    #[serde(default = "default_context_window_tokens")]
    pub(crate) context_window_tokens: u32,
}

impl Default for SessionConfigFile {
    fn default() -> Self {
        Self {
            auto_compact: default_session_auto_compact(),
            auto_compact_min_input_tokens: 0,
            auto_compact_ratio: default_auto_compact_ratio(),
            context_window_auto: default_context_window_auto(),
            context_window_tokens: default_context_window_tokens(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ModelProfile {
    /// 覆盖全局 `provider`（目录 id，如 `z.ai`、`anthropic`、`openrouter`）
    #[serde(default)]
    pub(crate) provider: Option<String>,
    /// 该 profile 专用 API Key（不填则按厂商从全局 `api_key` 或 `provider_credentials` 解析）
    #[serde(default)]
    pub(crate) api_key: Option<String>,
    /// 套餐：coding / general（不填则沿用全局 plan）
    #[serde(default)]
    pub(crate) plan: Option<String>,
    /// model id（不填则沿用全局 model）
    #[serde(default)]
    pub(crate) model: Option<String>,
    #[serde(default)]
    pub(crate) temperature: Option<f32>,
    #[serde(default)]
    pub(crate) max_tokens: Option<u32>,
    /// 覆盖 base_url（不填则沿用全局 base_url 或 plan 默认）
    #[serde(default)]
    pub(crate) base_url: Option<String>,
}

impl ModelProfile {
    // 预留：后续用于校验/合并 profile
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct RoutingConfig {
    /// 默认 profile（可选）
    #[serde(default)]
    pub(crate) default: Option<ModelProfile>,
    /// 按 agent_type 覆盖（如 plan/explore/general-purpose）
    #[serde(default)]
    pub(crate) agents: HashMap<String, ModelProfile>,
}

fn default_skills_enabled() -> bool {
    true
}

fn default_skill_run_timeout_ms() -> u64 {
    120_000
}

/// `config.json` 中的 `skills` 段（serde）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SkillsConfigFile {
    /// When false, no scan, no prompt injection, `Skill` tool resolves only cwd-based skills.
    #[serde(default = "default_skills_enabled")]
    pub(crate) enabled: bool,
    /// Extra roots scanned before `~/.anycode/skills` (lower precedence than user dir).
    #[serde(default)]
    pub(crate) extra_dirs: Vec<PathBuf>,
    /// If set, only these skill ids appear in the catalog and prompt.
    #[serde(default)]
    pub(crate) allowlist: Option<Vec<String>>,
    #[serde(default = "default_skill_run_timeout_ms")]
    pub(crate) run_timeout_ms: u64,
    /// Strip environment to a small whitelist for `Skill` tool subprocesses.
    #[serde(default)]
    pub(crate) minimal_env: bool,
    /// Also register `Skill` for explore/plan agents (default off).
    #[serde(default)]
    pub(crate) expose_on_explore_plan: bool,
}

impl Default for SkillsConfigFile {
    fn default() -> Self {
        Self {
            enabled: default_skills_enabled(),
            extra_dirs: vec![],
            allowlist: None,
            run_timeout_ms: default_skill_run_timeout_ms(),
            minimal_env: false,
            expose_on_explore_plan: false,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SkillsConfig {
    pub(crate) enabled: bool,
    pub(crate) extra_dirs: Vec<PathBuf>,
    pub(crate) allowlist: Option<Vec<String>>,
    pub(crate) run_timeout_ms: u64,
    pub(crate) minimal_env: bool,
    pub(crate) expose_on_explore_plan: bool,
}

impl From<SkillsConfigFile> for SkillsConfig {
    fn from(f: SkillsConfigFile) -> Self {
        Self {
            enabled: f.enabled,
            extra_dirs: f.extra_dirs,
            allowlist: f.allowlist,
            run_timeout_ms: f.run_timeout_ms,
            minimal_env: f.minimal_env,
            expose_on_explore_plan: f.expose_on_explore_plan,
        }
    }
}

impl Default for SkillsConfig {
    fn default() -> Self {
        SkillsConfigFile::default().into()
    }
}

fn default_runtime_mode() -> String {
    "code".to_string()
}

fn default_runtime_enabled_features() -> Vec<String> {
    vec![
        FeatureFlag::Skills.as_str().to_string(),
        FeatureFlag::ApprovalV2.as_str().to_string(),
        FeatureFlag::ContextCompression.as_str().to_string(),
        FeatureFlag::WorkspaceProfiles.as_str().to_string(),
        FeatureFlag::ChannelMode.as_str().to_string(),
    ]
}

fn default_runtime_model_routes() -> ModelRouteProfile {
    let mut mode_aliases = HashMap::new();
    mode_aliases.insert("general".to_string(), "code".to_string());
    mode_aliases.insert("explore".to_string(), "fast".to_string());
    mode_aliases.insert("plan".to_string(), "plan".to_string());
    mode_aliases.insert("code".to_string(), "code".to_string());
    mode_aliases.insert("channel".to_string(), "channel".to_string());
    mode_aliases.insert("goal".to_string(), "best".to_string());
    let mut agent_aliases = HashMap::new();
    agent_aliases.insert("summary".to_string(), "summary".to_string());
    ModelRouteProfile {
        default_alias: Some("code".to_string()),
        mode_aliases,
        agent_aliases,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RuntimeSettingsFile {
    #[serde(default = "default_runtime_mode")]
    pub(crate) default_mode: String,
    #[serde(default = "default_runtime_enabled_features")]
    pub(crate) enabled_features: Vec<String>,
    #[serde(default = "default_runtime_model_routes")]
    pub(crate) model_routes: ModelRouteProfile,
}

impl Default for RuntimeSettingsFile {
    fn default() -> Self {
        Self {
            default_mode: default_runtime_mode(),
            enabled_features: default_runtime_enabled_features(),
            model_routes: default_runtime_model_routes(),
        }
    }
}

/// 持久化到 ~/.anycode/config.json 的安全相关选项（与运行时 `SecurityConfig` 对应）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SecurityConfigFile {
    /// `default` | `auto` | `plan` | `bypass`
    #[serde(default = "default_security_permission_mode")]
    pub(crate) permission_mode: String,
    #[serde(default = "default_security_require_approval")]
    pub(crate) require_approval: bool,
    #[serde(default)]
    pub(crate) sandbox_mode: bool,
    /// 工具 API 名 deny 正则（例如 `^mcp__prod__`）；非法条目跳过并打日志。
    #[serde(default)]
    pub(crate) mcp_tool_deny_patterns: Vec<String>,
    /// MCP 工具 blanket deny 规则（非正则，见 `mcp_tool_deny_rules` 文档）。
    #[serde(default)]
    pub(crate) mcp_tool_deny_rules: Vec<String>,
    #[serde(default)]
    pub(crate) always_allow_rules: Vec<String>,
    #[serde(default)]
    pub(crate) always_ask_rules: Vec<String>,
    #[serde(default)]
    pub(crate) defer_mcp_tools: bool,
}

fn default_security_permission_mode() -> String {
    "default".to_string()
}

fn default_security_require_approval() -> bool {
    true
}

impl Default for SecurityConfigFile {
    fn default() -> Self {
        Self {
            permission_mode: default_security_permission_mode(),
            require_approval: default_security_require_approval(),
            sandbox_mode: false,
            mcp_tool_deny_patterns: vec![],
            mcp_tool_deny_rules: vec![],
            always_allow_rules: vec![],
            always_ask_rules: vec![],
            defer_mcp_tools: false,
        }
    }
}

pub(crate) fn validate_permission_mode(s: &str) -> anyhow::Result<()> {
    match s {
        "default" | "auto" | "plan" | "accept_edits" | "acceptEdits" | "bypass" => Ok(()),
        _ => {
            let mut a = FluentArgs::new();
            a.set("mode", s);
            anyhow::bail!("{}", tr_args("err-permission-mode", &a));
        }
    }
}

pub(crate) fn validate_runtime_mode(s: &str) -> anyhow::Result<RuntimeMode> {
    RuntimeMode::parse(s).ok_or_else(|| anyhow::anyhow!("invalid runtime mode: {}", s))
}

pub(crate) fn validate_llm_provider(s: &str) -> anyhow::Result<()> {
    let n = normalize_provider_id(s);
    if is_known_provider_id(&n) {
        return Ok(());
    }
    let mut a = FluentArgs::new();
    a.set("p", s);
    anyhow::bail!("{}", tr_args("err-provider", &a));
}

fn validate_qualified_model_ref(qualified: &str) -> anyhow::Result<()> {
    let (prov, mid) = qualified
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("internal: qualified model ref expected to contain '/'"))?;
    let mid = mid.trim();
    if mid.is_empty() {
        anyhow::bail!("{}", tr("err-model-required"));
    }
    let n = normalize_provider_id(prov);
    if !is_known_provider_id(&n) {
        let mut a = FluentArgs::new();
        a.set("p", prov);
        anyhow::bail!("{}", tr_args("err-provider", &a));
    }
    if is_zai_family_provider(&n) && !is_known_zai_model(mid) {
        let list = ZAI_MODEL_CATALOG
            .iter()
            .map(|e| e.api_name)
            .collect::<Vec<_>>()
            .join(", ");
        let mut a = FluentArgs::new();
        a.set("id", mid);
        a.set("list", list);
        anyhow::bail!("{}", tr_args("err-unknown-zai-model", &a));
    }
    Ok(())
}

/// `repl --model` 等仅本会话的模型覆盖：与 `model set` 的 z.ai 目录校验一致；Anthropic 允许任意非空 id；
/// 其它厂商须为已知 provider，model 为非空字符串。
/// OpenClaw 风格：若 `model` 含 `/`，按 `provider/model` 解析（与全局 `provider` 字段独立）。
pub(crate) fn validate_session_model_override(provider: &str, model: &str) -> anyhow::Result<()> {
    let m = model.trim();
    if m.is_empty() {
        anyhow::bail!("{}", tr("err-model-required"));
    }
    if m.contains('/') {
        return validate_qualified_model_ref(m);
    }
    if is_zai_family_provider(provider) {
        let cat = zai_model_catalog_entries();
        let r = resolve_chat_model_ref(m, Some(provider), &cat);
        if r.reason == Some(ChatModelResolutionReason::Ambiguous) {
            anyhow::bail!(
                "ambiguous model id {:?}: matches multiple catalog entries",
                m
            );
        }
        if !is_known_zai_model(m) {
            let list = ZAI_MODEL_CATALOG
                .iter()
                .map(|e| e.api_name)
                .collect::<Vec<_>>()
                .join(", ");
            let mut a = FluentArgs::new();
            a.set("id", m);
            a.set("list", list);
            anyhow::bail!("{}", tr_args("err-unknown-zai-model", &a));
        }
    } else if is_anthropic_family_provider(provider) {
        // 与配置文件一致，不强制枚举 Claude model id
    } else {
        validate_llm_provider(provider)?;
    }
    Ok(())
}

pub(crate) fn apply_optional_repl_model(
    config: &mut Config,
    model: Option<String>,
) -> anyhow::Result<()> {
    if let Some(m) = model {
        validate_session_model_override(&config.llm.provider, &m)?;
        config.llm.model = m;
    }
    Ok(())
}

/// 内联文本，或以 `@path` 从文件读取（相对路径相对 `base_dir`，通常为配置文件所在目录）。
pub(crate) fn resolve_system_prompt_field(raw: &str, base_dir: &Path) -> anyhow::Result<String> {
    if let Some(rest) = raw.strip_prefix('@') {
        let path_str = rest.trim();
        let p = if Path::new(path_str).is_absolute() {
            PathBuf::from(path_str)
        } else {
            base_dir.join(path_str)
        };
        fs::read_to_string(&p).with_context(|| {
            let mut a = FluentArgs::new();
            a.set("path", p.display().to_string());
            tr_args("err-read-system-prompt", &a)
        })
    } else {
        Ok(raw.to_string())
    }
}
