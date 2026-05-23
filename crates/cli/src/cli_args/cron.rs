use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum CronCommands {
    /// Print recent scheduler run ledger entries from ~/.anycode/logs/cron-runs.jsonl
    Runs {
        /// Filter by job id
        #[arg(long)]
        job: Option<String>,
        /// Filter by stable cron session id
        #[arg(long)]
        session: Option<String>,
        /// Maximum rows to print
        #[arg(long, default_value_t = 20)]
        limit: usize,
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
