use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum StatuslineCommands {
    /// Print example JSON payload (pretty-printed)
    PrintSchema,
}
