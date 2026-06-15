use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub(crate) enum AuthCommands {
    /// Open cloud portal login in browser
    Login,
    /// Complete device link with code from portal or anycode:// deep link
    Link {
        #[arg(long)]
        code: String,
    },
    /// Show linked cloud account status
    Status,
    /// Clear local cloud session
    Logout,
}
