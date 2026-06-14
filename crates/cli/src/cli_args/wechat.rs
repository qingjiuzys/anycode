use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum WechatCommands {
    /// Local encrypted DB chat history (not iLink bot channel)
    History {
        #[command(subcommand)]
        sub: WechatHistoryCommands,
    },
}

#[derive(Subcommand, Debug)]
pub(crate) enum WechatHistoryCommands {
    /// Install deps, extract DB keys, write config (one-shot)
    Setup {
        /// JSON output from underlying setup script
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Detect WeChat db_storage, keys, sqlcipher (read-only)
    Status {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
