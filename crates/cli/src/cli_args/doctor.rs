use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum DoctorCommands {
    /// Run all lightweight local diagnostics
    All {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Diagnose memory backend state
    Memory {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Diagnose channel bridge state
    Channel {
        /// Channel name: wechat / telegram / discord / all
        #[arg(default_value = "all")]
        channel: String,
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Diagnose MCP configuration and lifecycle policy
    Mcp {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Diagnose tool governance metadata
    Tools {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Diagnose local WeChat encrypted DB history readiness
    WechatHistory {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Print structured CLI error taxonomy reference
    Errors {
        /// JSON output
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}
