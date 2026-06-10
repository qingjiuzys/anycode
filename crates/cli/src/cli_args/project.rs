use std::path::PathBuf;

#[derive(clap::Subcommand, Debug)]
pub(crate) enum ProjectCommands {
    /// List built-in project templates
    Templates {
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Create a project from a template
    Init {
        /// Template id (e.g. flutter-app)
        #[arg(long)]
        template: String,
        /// Target directory (created if missing)
        #[arg(short = 'C', long)]
        path: PathBuf,
        /// Dart package name (snake_case)
        #[arg(long)]
        name: Option<String>,
        /// Display title (UI strings)
        #[arg(long)]
        title: Option<String>,
        /// Flutter bundle org (e.g. com.example.app)
        #[arg(long)]
        org: Option<String>,
        /// Replace non-empty target directory
        #[arg(long, default_value_t = false)]
        force: bool,
        /// Run `flutter create` when Flutter is already on PATH (default: agent-first skeleton only)
        #[arg(long, default_value_t = false)]
        flutter_create: bool,
    },
}
