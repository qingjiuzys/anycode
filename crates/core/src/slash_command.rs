//! Shared slash command metadata for prompts and local shells.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlashCommandScope {
    Local,
    Runtime,
    PromptOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub name: &'static str,
    pub summary: &'static str,
    pub scope: SlashCommandScope,
}

pub const BUILTIN_SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        name: "mode",
        summary: "切换当前运行模式",
        scope: SlashCommandScope::Local,
    },
    SlashCommand {
        name: "model",
        summary: "查看或切换当前模型别名",
        scope: SlashCommandScope::Local,
    },
    SlashCommand {
        name: "status",
        summary: "查看当前 workspace、模式、模型与审批状态",
        scope: SlashCommandScope::Runtime,
    },
    SlashCommand {
        name: "compact",
        summary: "触发上下文压缩",
        scope: SlashCommandScope::Runtime,
    },
    SlashCommand {
        name: "memory",
        summary: "查看或操作记忆",
        scope: SlashCommandScope::Runtime,
    },
    SlashCommand {
        name: "approve",
        summary: "处理待审批操作",
        scope: SlashCommandScope::PromptOnly,
    },
    SlashCommand {
        name: "workflow",
        summary: "查看或加载 workflow",
        scope: SlashCommandScope::Runtime,
    },
];
