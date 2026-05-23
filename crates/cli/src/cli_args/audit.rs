use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum AuditCommands {
    /// Print recent tool audit entries from ~/.anycode/audit/tool-calls.jsonl
    Tail {
        /// Filter by task id
        #[arg(long)]
        task: Option<String>,
        /// Filter by tool name
        #[arg(long)]
        tool: Option<String>,
        /// Maximum rows to print
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
