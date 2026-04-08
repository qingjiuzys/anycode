//! OpenClaw-style chat model references: `provider/model` vs catalog resolution.
//!
//! Mirrors `openclaw/ui/src/ui/chat-model-ref.ts` (`buildQualifiedChatModelValue`,
//! `resolveChatModelOverride`).

/// Single catalog row: model id + owning provider id (normalized).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelCatalogEntry<'a> {
    pub id: &'a str,
    pub provider: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatModelResolutionSource {
    Empty,
    Qualified,
    Catalog,
    Raw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatModelResolutionReason {
    Empty,
    Missing,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatModelResolution {
    /// Qualified or best-effort display value (`provider/model` or raw id).
    pub value: String,
    pub source: ChatModelResolutionSource,
    pub reason: Option<ChatModelResolutionReason>,
}

impl ChatModelResolution {
    fn empty() -> Self {
        Self {
            value: String::new(),
            source: ChatModelResolutionSource::Empty,
            reason: Some(ChatModelResolutionReason::Empty),
        }
    }
}

/// If `model` already contains `/`, return as-is; else join with `provider` when present.
pub fn build_qualified_chat_model_value(model: &str, provider: Option<&str>) -> String {
    let trimmed = model.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.contains('/') {
        return trimmed.to_string();
    }
    let p = provider.map(str::trim).filter(|s| !s.is_empty());
    match p {
        Some(prov) => format!("{prov}/{trimmed}"),
        None => trimmed.to_string(),
    }
}

/// Resolve a user/model string: qualified passthrough; otherwise match `catalog` by id (case-insensitive).
pub fn resolve_chat_model_ref(
    raw: &str,
    default_provider: Option<&str>,
    catalog: &[ModelCatalogEntry<'_>],
) -> ChatModelResolution {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return ChatModelResolution::empty();
    }
    if trimmed.contains('/') {
        return ChatModelResolution {
            value: trimmed.to_string(),
            source: ChatModelResolutionSource::Qualified,
            reason: None,
        };
    }

    let mut matched: Option<String> = None;
    for entry in catalog {
        if entry.id.trim().eq_ignore_ascii_case(trimmed) {
            let candidate = build_qualified_chat_model_value(entry.id, Some(entry.provider));
            if matched.is_none() {
                matched = Some(candidate);
            } else if matched.as_ref().map(|s| s.to_ascii_lowercase())
                != Some(candidate.to_ascii_lowercase())
            {
                return ChatModelResolution {
                    value: trimmed.to_string(),
                    source: ChatModelResolutionSource::Raw,
                    reason: Some(ChatModelResolutionReason::Ambiguous),
                };
            }
        }
    }

    if let Some(v) = matched {
        return ChatModelResolution {
            value: v,
            source: ChatModelResolutionSource::Catalog,
            reason: None,
        };
    }

    ChatModelResolution {
        value: build_qualified_chat_model_value(trimmed, default_provider),
        source: ChatModelResolutionSource::Raw,
        reason: Some(ChatModelResolutionReason::Missing),
    }
}

/// z.ai Coding catalog as OpenClaw-style entries (`provider` = normalized `z.ai`).
pub fn zai_model_catalog_entries() -> Vec<ModelCatalogEntry<'static>> {
    crate::providers::zai::ZAI_MODEL_CATALOG
        .iter()
        .map(|e| ModelCatalogEntry {
            id: e.api_name,
            provider: "z.ai",
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_catalog() -> Vec<ModelCatalogEntry<'static>> {
        vec![
            ModelCatalogEntry {
                id: "glm-5",
                provider: "z.ai",
            },
            ModelCatalogEntry {
                id: "glm-4",
                provider: "z.ai",
            },
        ]
    }

    #[test]
    fn qualified_passthrough() {
        let r = resolve_chat_model_ref("anthropic/claude-3-5-sonnet", Some("z.ai"), &sample_catalog());
        assert_eq!(r.source, ChatModelResolutionSource::Qualified);
        assert_eq!(r.value, "anthropic/claude-3-5-sonnet");
    }

    #[test]
    fn catalog_resolves_case_insensitive() {
        let r = resolve_chat_model_ref("GLM-5", Some("z.ai"), &sample_catalog());
        assert_eq!(r.source, ChatModelResolutionSource::Catalog);
        assert_eq!(r.value, "z.ai/glm-5");
    }

    #[test]
    fn raw_unknown_gets_default_provider_prefix() {
        let r = resolve_chat_model_ref("custom-model", Some("openrouter"), &[]);
        assert_eq!(r.source, ChatModelResolutionSource::Raw);
        assert_eq!(r.reason, Some(ChatModelResolutionReason::Missing));
        assert_eq!(r.value, "openrouter/custom-model");
    }

    #[test]
    fn build_qualified_skips_double_provider() {
        assert_eq!(
            build_qualified_chat_model_value("z.ai/glm-5", Some("ignored")),
            "z.ai/glm-5"
        );
    }

    #[test]
    fn ambiguous_catalog_ids() {
        let catalog = vec![
            ModelCatalogEntry {
                id: "dup",
                provider: "z.ai",
            },
            ModelCatalogEntry {
                id: "dup",
                provider: "other",
            },
        ];
        let r = resolve_chat_model_ref("dup", None, &catalog);
        assert_eq!(r.source, ChatModelResolutionSource::Raw);
        assert_eq!(r.reason, Some(ChatModelResolutionReason::Ambiguous));
    }
}
