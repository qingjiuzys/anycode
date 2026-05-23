use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum ModelCommands {
    /// List models (z.ai static catalog for now)
    #[command(hide = true)]
    List {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,

        /// Plain output (one model id per line)
        #[arg(long, default_value_t = false)]
        plain: bool,
    },

    /// Show configured model status
    #[command(hide = true)]
    Status {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },

    /// Set default model
    #[command(hide = true)]
    Set {
        /// Model id (e.g. glm-5)
        model: String,
    },

    /// OAuth / token helpers for model providers
    Auth {
        #[command(subcommand)]
        sub: ModelAuthCommands,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum ModelAuthCommands {
    /// GitHub device flow; writes ~/.anycode/credentials/github-oauth.json for GitHub Copilot
    Copilot,
}
