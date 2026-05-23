use super::*;

#[derive(Subcommand, Debug)]
pub(crate) enum SkillsCommands {
    /// List discovered skills (id, description, has `run`, root path)
    List,
    /// Print effective skill search roots (extra_dirs then ~/.anycode/skills)
    Path,
    /// Create ~/.anycode/skills/<name>/ with minimal SKILL.md and `run` template
    Init {
        /// Skill id (directory name): letters, digits, `.`, `_`, `-` only
        name: String,
    },
}
