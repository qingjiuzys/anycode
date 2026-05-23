//! Config validation and prompt field resolution helpers.

use super::{is_anthropic_family_provider, is_known_zai_model, is_zai_family_provider, Config};
use crate::i18n::{tr, tr_args};
use anycode_core::RuntimeMode;
use anycode_llm::{
    is_known_provider_id, normalize_provider_id, resolve_chat_model_ref, zai_model_catalog_entries,
    ChatModelResolutionReason, ZAI_MODEL_CATALOG,
};
use anyhow::Context;
use fluent_bundle::FluentArgs;
use std::fs;
use std::path::{Path, PathBuf};

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

/// `config.json` `notifications`：非空 `http_url` 须为可解析的 `http`/`https`；`max_body_bytes` 有上下限。
pub(crate) fn validate_notifications(
    s: &anycode_core::SessionNotificationSettings,
) -> anyhow::Result<()> {
    const MIN_BODY: usize = 256;
    const MAX_BODY: usize = 512 * 1024;
    if s.max_body_bytes < MIN_BODY || s.max_body_bytes > MAX_BODY {
        anyhow::bail!(
            "notifications: max_body_bytes must be between {} and {} (got {})",
            MIN_BODY,
            MAX_BODY,
            s.max_body_bytes
        );
    }
    if let Some(raw) = s
        .http_url
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
    {
        let parsed = reqwest::Url::parse(raw).map_err(|e| {
            anyhow::anyhow!(
                "notifications: http_url is not a valid URL ({e}); check the value is a full http(s) address"
            )
        })?;
        let scheme = parsed.scheme();
        if scheme != "http" && scheme != "https" {
            anyhow::bail!(
                "notifications: http_url must use http or https (got scheme {:?})",
                scheme
            );
        }
    }
    Ok(())
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
