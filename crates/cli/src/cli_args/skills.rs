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
    /// Install skill(s) from a local path or git URL into ~/.anycode/skills/
    Install {
        /// Local directory or git clone URL
        source: String,
    },
    /// Scan a skill's `run` script for risky patterns (skill-vetter style)
    Vet {
        /// Skill id under configured roots
        id: String,
    },
    /// Copy bundled office skills from the repo into ~/.anycode/skills/
    InstallStarter,
}
