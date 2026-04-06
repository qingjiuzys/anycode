//! User-facing model aliases and default routing semantics.

use anycode_core::{ModelConfig, RuntimeMode};

#[derive(Debug, Clone, Copy)]
pub struct ModelAliasDescriptor {
    pub alias: &'static str,
    pub description: &'static str,
    pub mode: Option<RuntimeMode>,
}

pub const MODEL_ALIASES: &[ModelAliasDescriptor] = &[
    ModelAliasDescriptor {
        alias: "best",
        description: "最高质量默认模型",
        mode: None,
    },
    ModelAliasDescriptor {
        alias: "fast",
        description: "偏低成本/快速模型",
        mode: Some(RuntimeMode::Explore),
    },
    ModelAliasDescriptor {
        alias: "plan",
        description: "规划模式模型",
        mode: Some(RuntimeMode::Plan),
    },
    ModelAliasDescriptor {
        alias: "code",
        description: "编码模式模型",
        mode: Some(RuntimeMode::Code),
    },
    ModelAliasDescriptor {
        alias: "channel",
        description: "通道/Workspace 助手模型",
        mode: Some(RuntimeMode::Channel),
    },
    ModelAliasDescriptor {
        alias: "summary",
        description: "总结/压缩模型",
        mode: None,
    },
];

pub fn is_known_model_alias(alias: &str) -> bool {
    let normalized = alias.trim().to_ascii_lowercase();
    MODEL_ALIASES.iter().any(|item| item.alias == normalized)
}

pub fn known_model_aliases() -> Vec<&'static str> {
    MODEL_ALIASES.iter().map(|item| item.alias).collect()
}

pub fn clone_with_model(config: &ModelConfig, model: impl Into<String>) -> ModelConfig {
    let mut cloned = config.clone();
    cloned.model = model.into();
    cloned
}
