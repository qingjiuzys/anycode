use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum EvalCommands {
    /// List built-in production readiness scenarios
    List {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Run the built-in eval scenarios that do not require real provider credentials
    Run {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
        /// Also run mock-LLM fixture repo task (local TCP mock; nightly/CI extended set)
        #[arg(long, default_value_t = false)]
        mock: bool,
    },
}
