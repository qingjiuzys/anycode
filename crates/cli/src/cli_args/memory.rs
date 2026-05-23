use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum MemoryCommands {
    /// Import Markdown memories from `memory.path` into pipeline hot (Sled) store
    Import {
        /// Print actions without writing
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Maximum number of memories to import (across all types)
        #[arg(long)]
        limit: Option<usize>,
    },
    /// Dry-run or apply memory retention pruning
    Prune {
        /// Print actions without deleting
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        /// Apply the prune plan
        #[arg(long, default_value_t = false)]
        apply: bool,
        /// Retain memories updated within this many days
        #[arg(long, default_value_t = 90)]
        older_than_days: i64,
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Diagnose configured memory paths and common lock/vector issues
    Doctor {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
