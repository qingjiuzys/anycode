//! Structured CLI error taxonomy for ops-friendly exits.

use anyhow::Error;
use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CliErrorCategory {
    Config,
    Runtime,
    Channel,
    Mcp,
    Eval,
    UserInput,
    Internal,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ClassifiedCliError {
    pub category: CliErrorCategory,
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
    pub exit_code: u8,
}

pub(crate) fn errors_json_enabled() -> bool {
    match std::env::var("ANYCODE_ERRORS_JSON") {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

pub(crate) fn classify(err: &Error) -> ClassifiedCliError {
    let message = err.to_string();
    let lower = message.to_ascii_lowercase();
    let chain = err
        .chain()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join(" | ");
    let hay = format!("{lower} {chain}").to_ascii_lowercase();

    let (category, code, hint, exit_code) = if hay.contains("eval harness") {
        (
            CliErrorCategory::Eval,
            "eval.scenario_failed",
            Some("Run `anycode eval list` and fix failing scenarios.".into()),
            2,
        )
    } else if hay.contains("no config")
        || hay.contains("config.json")
        || hay.contains("parse config")
        || hay.contains("load config")
    {
        (
            CliErrorCategory::Config,
            "config.invalid_or_missing",
            Some("Run `anycode config` or check ~/.anycode/config.json.".into()),
            3,
        )
    } else if hay.contains("telegram")
        || hay.contains("discord")
        || hay.contains("wechat")
        || hay.contains("wx ")
        || hay.contains("channel")
    {
        (
            CliErrorCategory::Channel,
            "channel.bridge",
            Some("See `anycode doctor channel --json`.".into()),
            4,
        )
    } else if hay.contains("mcp") || hay.contains("oauth") {
        (
            CliErrorCategory::Mcp,
            "mcp.lifecycle",
            Some("See `anycode mcp status --json` and ADR 007 reconnect policy.".into()),
            5,
        )
    } else if hay.contains("cancelled") || hay.contains("cooperative") {
        (CliErrorCategory::UserInput, "user.cancelled", None, 130)
    } else if hay.contains("memory")
        || hay.contains("sled")
        || hay.contains("wouldblock")
        || hay.contains("embedding")
    {
        (
            CliErrorCategory::Runtime,
            "runtime.memory",
            Some("Run `anycode memory doctor --json`.".into()),
            6,
        )
    } else if hay.contains("llm") || hay.contains("provider") || hay.contains("api key") {
        (
            CliErrorCategory::Runtime,
            "runtime.llm",
            Some("Check model credentials and `anycode status --json`.".into()),
            7,
        )
    } else {
        (
            CliErrorCategory::Internal,
            "internal.unclassified",
            Some("Re-run with `ANYCODE_ERRORS_JSON=1` for structured output.".into()),
            1,
        )
    };

    ClassifiedCliError {
        category,
        code: code.into(),
        message,
        hint,
        exit_code,
    }
}

pub(crate) fn emit_and_exit(err: &Error) -> ! {
    let row = classify(err);
    if errors_json_enabled() {
        eprintln!(
            "{}",
            serde_json::to_string_pretty(&row).unwrap_or_else(|_| row.message.clone())
        );
    } else {
        eprintln!(
            "error [{}:{}] {}",
            row.code,
            category_label(row.category),
            row.message
        );
        if let Some(h) = row.hint {
            eprintln!("hint: {h}");
        }
    }
    std::process::exit(i32::from(row.exit_code));
}

fn category_label(c: CliErrorCategory) -> &'static str {
    match c {
        CliErrorCategory::Config => "config",
        CliErrorCategory::Runtime => "runtime",
        CliErrorCategory::Channel => "channel",
        CliErrorCategory::Mcp => "mcp",
        CliErrorCategory::Eval => "eval",
        CliErrorCategory::UserInput => "user",
        CliErrorCategory::Internal => "internal",
    }
}

#[derive(Debug, Clone, Serialize)]
struct TaxonomyRow {
    category: CliErrorCategory,
    code: String,
    description: String,
}

pub(crate) fn taxonomy_rows() -> Vec<TaxonomyRow> {
    vec![
        TaxonomyRow {
            category: CliErrorCategory::Config,
            code: "config.invalid_or_missing".into(),
            description: "Config file missing, unreadable, or failed schema validation.".into(),
        },
        TaxonomyRow {
            category: CliErrorCategory::Runtime,
            code: "runtime.llm".into(),
            description: "LLM provider transport, auth, or model routing failure.".into(),
        },
        TaxonomyRow {
            category: CliErrorCategory::Runtime,
            code: "runtime.memory".into(),
            description: "Memory backend lock, embedding, or pipeline errors.".into(),
        },
        TaxonomyRow {
            category: CliErrorCategory::Channel,
            code: "channel.bridge".into(),
            description: "WeChat / Telegram / Discord bridge setup or delivery errors.".into(),
        },
        TaxonomyRow {
            category: CliErrorCategory::Mcp,
            code: "mcp.lifecycle".into(),
            description: "MCP env, OAuth, or controlled reconnect policy violations.".into(),
        },
        TaxonomyRow {
            category: CliErrorCategory::Eval,
            code: "eval.scenario_failed".into(),
            description: "Production eval harness scenario did not meet acceptance.".into(),
        },
        TaxonomyRow {
            category: CliErrorCategory::UserInput,
            code: "user.cancelled".into(),
            description: "Cooperative cancel or explicit user abort.".into(),
        },
        TaxonomyRow {
            category: CliErrorCategory::Internal,
            code: "internal.unclassified".into(),
            description: "Fallback when no heuristic matches; file an issue with logs.".into(),
        },
    ]
}

pub(crate) fn print_taxonomy(json: bool) -> anyhow::Result<()> {
    let rows = taxonomy_rows();
    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
    } else {
        for row in rows {
            println!("{} — {} ({:?})", row.code, row.description, row.category);
        }
        println!("\nSet ANYCODE_ERRORS_JSON=1 to emit structured errors on CLI failure.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_eval_failures() {
        let err = anyhow::anyhow!("eval harness: one or more scenarios failed");
        let row = classify(&err);
        assert_eq!(row.category, CliErrorCategory::Eval);
        assert_eq!(row.code, "eval.scenario_failed");
    }

    #[test]
    fn classifies_config_errors() {
        let err = anyhow::anyhow!("failed to parse config.json");
        let row = classify(&err);
        assert_eq!(row.category, CliErrorCategory::Config);
    }
}
