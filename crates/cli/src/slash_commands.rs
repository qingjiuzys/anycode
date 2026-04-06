use anycode_core::{SlashCommand, SlashCommandScope, BUILTIN_SLASH_COMMANDS};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedSlashCommand {
    Mode(Option<String>),
    Model(Option<String>),
    Status,
    Compact,
    Memory,
    Approve,
    Workflow(Option<String>),
}

pub fn registry() -> &'static [SlashCommand] {
    BUILTIN_SLASH_COMMANDS
}

pub fn help_lines() -> Vec<String> {
    registry()
        .iter()
        .map(|cmd| {
            let scope = match cmd.scope {
                SlashCommandScope::Local => "local",
                SlashCommandScope::Runtime => "runtime",
                SlashCommandScope::PromptOnly => "prompt",
            };
            format!("/{:<10} {:<7} {}", cmd.name, scope, cmd.summary)
        })
        .collect()
}

pub fn parse(input: &str) -> Option<ParsedSlashCommand> {
    let trimmed = input.trim();
    let body = trimmed.strip_prefix('/')?;
    let mut parts = body.split_whitespace();
    let cmd = parts.next()?.to_ascii_lowercase();
    let rest = parts.collect::<Vec<_>>().join(" ");
    let arg = if rest.trim().is_empty() {
        None
    } else {
        Some(rest)
    };
    match cmd.as_str() {
        "mode" => Some(ParsedSlashCommand::Mode(arg)),
        "model" => Some(ParsedSlashCommand::Model(arg)),
        "status" => Some(ParsedSlashCommand::Status),
        "compact" => Some(ParsedSlashCommand::Compact),
        "memory" => Some(ParsedSlashCommand::Memory),
        "approve" => Some(ParsedSlashCommand::Approve),
        "workflow" => Some(ParsedSlashCommand::Workflow(arg)),
        _ => None,
    }
}
