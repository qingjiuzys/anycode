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
        name: "workflow",
        summary: "查看或加载 workflow",
        scope: SlashCommandScope::Runtime,
    },
    SlashCommand {
        name: "session",
        summary: "恢复会话：无参按当前目录优先；list 列出；可跟 uuid",
        scope: SlashCommandScope::Runtime,
    },
    SlashCommand {
        name: "context",
        summary: "只读：消息条数、上下文窗口与上一轮 token 用量（若有）",
        scope: SlashCommandScope::Runtime,
    },
    SlashCommand {
        name: "export",
        summary: "将当前会话消息导出为纯文本；可选路径，默认 anycode-export-<id>.txt",
        scope: SlashCommandScope::Runtime,
    },
    SlashCommand {
        name: "cost",
        summary: "只读：上一轮 token 用量摘要（不提供美元计费；账单以提供商为准）",
        scope: SlashCommandScope::Runtime,
    },
];
