use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum WorkflowCommands {
    /// Validate a workflow YAML file without executing it
    Validate {
        /// Workflow file path
        file: PathBuf,
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
