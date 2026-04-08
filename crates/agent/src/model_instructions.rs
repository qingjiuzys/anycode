//! Model instructions file discovery and loading.
//!
//! Searches for `AGENTS.md` (or configured filename) in:
//! 1. Working directory
//! 2. `.anycode/` subdirectory
//! 3. Parent directories up to project root (stops at `.git` or filesystem root)
//!
//! The first file found is loaded and appended to the system prompt.

use std::fs;
use std::path::{Path, PathBuf};

/// Default filename for model instructions.
pub const DEFAULT_MODEL_INSTRUCTIONS_FILENAME: &str = "AGENTS.md";

/// Alternative filenames to search (in order of preference).
pub const MODEL_INSTRUCTIONS_FILENAMES: &[&str] = &[
    "AGENTS.md",
    ".agents.md",
    "agents.md",
    "MODEL_INSTRUCTIONS.md",
    ".model_instructions.md",
];

/// Search result containing the file path and its contents.
#[derive(Debug, Clone)]
pub struct ModelInstructionsFile {
    pub path: PathBuf,
    pub content: String,
}

/// Configuration for model instructions file discovery.
#[derive(Debug, Clone, Default)]
pub struct ModelInstructionsConfig {
    /// Whether to enable model instructions file discovery.
    pub enabled: bool,
    /// Custom filename to search for (if None, uses default search order).
    pub filename: Option<String>,
    /// Maximum number of parent directories to traverse.
    pub max_depth: Option<usize>,
}

impl ModelInstructionsConfig {
    pub fn new() -> Self {
        Self {
            enabled: true,
            filename: None,
            max_depth: Some(10),
        }
    }

    pub fn disabled() -> Self {
        Self {
            enabled: false,
            filename: None,
            max_depth: None,
        }
    }
}

/// Discover and load model instructions file from the working directory or its ancestors.
///
/// Returns `None` if no file is found or if discovery is disabled.
pub fn discover_model_instructions(
    working_directory: &str,
    config: &ModelInstructionsConfig,
) -> Option<ModelInstructionsFile> {
    if !config.enabled || working_directory.is_empty() {
        return None;
    }

    let cwd = Path::new(working_directory);
    if !cwd.is_absolute() || !cwd.exists() {
        return None;
    }

    let filenames: Vec<&str> = if let Some(ref custom) = config.filename {
        vec![custom.as_str()]
    } else {
        MODEL_INSTRUCTIONS_FILENAMES.to_vec()
    };

    let max_depth = config.max_depth.unwrap_or(10);
    let mut current = cwd.to_path_buf();
    let mut depth = 0;

    loop {
        if depth > max_depth {
            break;
        }

        // Search in current directory
        for filename in &filenames {
            let candidate = current.join(filename);
            if let Some(result) = try_load_file(&candidate) {
                return Some(result);
            }
        }

        // Search in .anycode/ subdirectory
        let anycode_dir = current.join(".anycode");
        if anycode_dir.is_dir() {
            for filename in &filenames {
                let candidate = anycode_dir.join(filename);
                if let Some(result) = try_load_file(&candidate) {
                    return Some(result);
                }
            }
        }

        // Stop at project root markers
        if is_project_root(&current) {
            break;
        }

        // Move to parent
        match current.parent() {
            Some(parent) if parent != current => {
                current = parent.to_path_buf();
                depth += 1;
            }
            _ => break,
        }
    }

    None
}

/// Try to load a file if it exists and is readable.
fn try_load_file(path: &Path) -> Option<ModelInstructionsFile> {
    if !path.is_file() {
        return None;
    }

    match fs::read_to_string(path) {
        Ok(content) => {
            let trimmed = content.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(ModelInstructionsFile {
                    path: path.to_path_buf(),
                    content: trimmed.to_string(),
                })
            }
        }
        Err(_) => None,
    }
}

/// Check if a directory is a project root (contains markers like .git).
fn is_project_root(dir: &Path) -> bool {
    const ROOT_MARKERS: &[&str] = &[
        ".git",
        ".hg",
        ".svn",
        "Cargo.toml",
        "package.json",
        "go.mod",
    ];

    for marker in ROOT_MARKERS {
        if dir.join(marker).exists() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_dir() -> TempDir {
        TempDir::new().expect("failed to create temp dir")
    }

    #[test]
    fn discover_disabled_returns_none() {
        let tmp = create_test_dir();
        let config = ModelInstructionsConfig::disabled();
        let result = discover_model_instructions(tmp.path().to_str().unwrap(), &config);
        assert!(result.is_none());
    }

    #[test]
    fn discover_empty_dir_returns_none() {
        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions("", &config);
        assert!(result.is_none());
    }

    #[test]
    fn discover_nonexistent_dir_returns_none() {
        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions("/nonexistent/path/12345", &config);
        assert!(result.is_none());
    }

    #[test]
    fn discover_agents_md_in_cwd() {
        let tmp = create_test_dir();
        let agents_path = tmp.path().join("AGENTS.md");
        let mut f = File::create(&agents_path).unwrap();
        writeln!(f, "# Test Instructions\nBe helpful.").unwrap();

        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions(tmp.path().to_str().unwrap(), &config);

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.path, agents_path);
        assert!(r.content.contains("Test Instructions"));
    }

    #[test]
    fn discover_in_anycode_subdir() {
        let tmp = create_test_dir();
        let anycode_dir = tmp.path().join(".anycode");
        fs::create_dir_all(&anycode_dir).unwrap();
        let agents_path = anycode_dir.join("AGENTS.md");
        let mut f = File::create(&agents_path).unwrap();
        writeln!(f, "# Subdirectory Instructions").unwrap();

        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions(tmp.path().to_str().unwrap(), &config);

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.path, agents_path);
    }

    #[test]
    fn discover_prefers_cwd_over_anycode_subdir() {
        let tmp = create_test_dir();

        // Create in .anycode/
        let anycode_dir = tmp.path().join(".anycode");
        fs::create_dir_all(&anycode_dir).unwrap();
        let anycode_path = anycode_dir.join("AGENTS.md");
        let mut f = File::create(&anycode_path).unwrap();
        writeln!(f, "# Subdirectory").unwrap();

        // Create in cwd
        let cwd_path = tmp.path().join("AGENTS.md");
        let mut f2 = File::create(&cwd_path).unwrap();
        writeln!(f2, "# CWD").unwrap();

        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions(tmp.path().to_str().unwrap(), &config);

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.path, cwd_path);
    }

    #[test]
    fn discover_parent_directory() {
        let tmp = create_test_dir();
        let sub_dir = tmp.path().join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();

        // Create in parent
        let parent_path = tmp.path().join("AGENTS.md");
        let mut f = File::create(&parent_path).unwrap();
        writeln!(f, "# Parent Instructions").unwrap();

        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions(sub_dir.to_str().unwrap(), &config);

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.path, parent_path);
    }

    #[test]
    fn discover_stops_at_git_root() {
        let tmp = create_test_dir();

        // Create .git marker
        fs::create_dir_all(tmp.path().join(".git")).unwrap();

        // Create AGENTS.md above the .git
        let sub_dir = tmp.path().join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();

        // File in parent (should be found since .git is in parent)
        let parent_path = tmp.path().join("AGENTS.md");
        let mut f = File::create(&parent_path).unwrap();
        writeln!(f, "# Root Instructions").unwrap();

        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions(sub_dir.to_str().unwrap(), &config);

        assert!(result.is_some());
    }

    #[test]
    fn discover_empty_file_returns_none() {
        let tmp = create_test_dir();
        let agents_path = tmp.path().join("AGENTS.md");
        File::create(&agents_path).unwrap(); // Empty file

        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions(tmp.path().to_str().unwrap(), &config);

        assert!(result.is_none());
    }

    #[test]
    fn discover_whitespace_only_file_returns_none() {
        let tmp = create_test_dir();
        let agents_path = tmp.path().join("AGENTS.md");
        let mut f = File::create(&agents_path).unwrap();
        writeln!(f, "   \n\n   \t  ").unwrap();

        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions(tmp.path().to_str().unwrap(), &config);

        assert!(result.is_none());
    }

    #[test]
    fn discover_custom_filename() {
        let tmp = create_test_dir();
        let custom_path = tmp.path().join("CUSTOM_RULES.md");
        let mut f = File::create(&custom_path).unwrap();
        writeln!(f, "# Custom Rules").unwrap();

        let config = ModelInstructionsConfig {
            enabled: true,
            filename: Some("CUSTOM_RULES.md".to_string()),
            max_depth: Some(10),
        };
        let result = discover_model_instructions(tmp.path().to_str().unwrap(), &config);

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.path, custom_path);
    }

    #[test]
    fn discover_alternative_filenames() {
        let tmp = create_test_dir();
        let alt_path = tmp.path().join(".agents.md");
        let mut f = File::create(&alt_path).unwrap();
        writeln!(f, "# Alt Instructions").unwrap();

        let config = ModelInstructionsConfig::new();
        let result = discover_model_instructions(tmp.path().to_str().unwrap(), &config);

        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.path, alt_path);
    }
}
