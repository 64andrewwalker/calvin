//! Project State Detection
//!
//! Detects the state of a Calvin project for the interactive command.

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetCount {
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectState {
    /// No .promptpack/ in project AND no user layer
    NoPromptPack,
    /// No .promptpack/ in project, but user layer exists at ~/.calvin/.promptpack
    UserLayerOnly(AssetCount),
    /// .promptpack/ exists but is empty
    EmptyPromptPack,
    /// .promptpack/ exists with assets
    Configured(AssetCount),
}

/// Get the user layer path (~/.calvin/.promptpack)
fn get_user_layer_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|home| home.join(".calvin").join(".promptpack"))
}

pub fn detect_state(cwd: &Path) -> ProjectState {
    let promptpack_dir = cwd.join(".promptpack");

    // First check project layer
    if promptpack_dir.is_dir() {
        let total = count_prompt_markdown_files(&promptpack_dir);
        if total == 0 {
            return ProjectState::EmptyPromptPack;
        } else {
            return ProjectState::Configured(AssetCount { total });
        }
    }

    // No project layer - check for user layer
    if let Some(user_layer) = get_user_layer_path() {
        if user_layer.is_dir() {
            let total = count_prompt_markdown_files(&user_layer);
            if total > 0 {
                return ProjectState::UserLayerOnly(AssetCount { total });
            }
        }
    }

    ProjectState::NoPromptPack
}

fn count_prompt_markdown_files(root: &Path) -> usize {
    fn walk(dir: &Path, total: &mut usize) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let is_hidden = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('.'));
                if is_hidden {
                    continue;
                }
                walk(&path, total);
                continue;
            }

            if path.extension().is_some_and(|ext| ext == "md")
                && path.file_name() != Some(std::ffi::OsStr::new("README.md"))
            {
                *total += 1;
            }
        }
    }

    let mut total = 0;
    walk(root, &mut total);
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_detect_state_no_promptpack() {
        let dir = tempdir().unwrap();
        let state = detect_state(dir.path());
        // When no project .promptpack exists, we get either:
        // - NoPromptPack (if no user layer exists)
        // - UserLayerOnly (if user layer exists at ~/.calvin/.promptpack)
        assert!(
            matches!(
                state,
                ProjectState::NoPromptPack | ProjectState::UserLayerOnly(_)
            ),
            "Expected NoPromptPack or UserLayerOnly, got {:?}",
            state
        );
    }

    #[test]
    fn test_detect_state_empty_promptpack() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".promptpack")).unwrap();
        assert_eq!(detect_state(dir.path()), ProjectState::EmptyPromptPack);
    }

    #[test]
    fn test_detect_state_ignores_readme_md() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(&promptpack).unwrap();
        std::fs::write(promptpack.join("README.md"), "# PromptPack\n").unwrap();
        assert_eq!(detect_state(dir.path()), ProjectState::EmptyPromptPack);
    }

    #[test]
    fn test_detect_state_configured_counts_assets() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(promptpack.join("actions")).unwrap();
        std::fs::write(
            promptpack.join("actions/test.md"),
            "---\ndescription: Test\n---\nBody\n",
        )
        .unwrap();

        assert_eq!(
            detect_state(dir.path()),
            ProjectState::Configured(AssetCount { total: 1 })
        );
    }

    #[test]
    fn test_detect_state_counts_nested_assets() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(promptpack.join("policies/nested")).unwrap();
        std::fs::write(
            promptpack.join("policies/nested/rule.md"),
            "---\ndescription: Rule\n---\nBody\n",
        )
        .unwrap();

        assert_eq!(
            detect_state(dir.path()),
            ProjectState::Configured(AssetCount { total: 1 })
        );
    }

    #[test]
    fn test_detect_state_skips_hidden_directories() {
        let dir = tempdir().unwrap();
        let promptpack = dir.path().join(".promptpack");
        std::fs::create_dir_all(promptpack.join(".hidden")).unwrap();
        std::fs::write(
            promptpack.join(".hidden/secret.md"),
            "---\ndescription: Secret\n---\nBody\n",
        )
        .unwrap();

        assert_eq!(detect_state(dir.path()), ProjectState::EmptyPromptPack);
    }
}
