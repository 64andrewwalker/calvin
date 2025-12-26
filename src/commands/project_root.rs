use std::path::{Path, PathBuf};

/// Discover the project root directory from an invocation directory.
///
/// Heuristics (first match wins, walking upward from `start`):
/// - `calvin.lock` (anchors to an existing deployment state)
/// - `.promptpack/` (default project layer location)
/// - `.git/` or `.git` file (git repo root / worktree)
///
/// Falls back to `start` when no markers are found.
pub(crate) fn discover_project_root(start: &Path) -> PathBuf {
    for dir in start.ancestors() {
        if dir.join("calvin.lock").exists() {
            return dir.to_path_buf();
        }
        if dir.join(".promptpack").is_dir() {
            return dir.to_path_buf();
        }
        if dir.join(".git").exists() {
            return dir.to_path_buf();
        }
    }
    start.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn discover_project_root_prefers_nearest_promptpack() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("sub/.promptpack")).unwrap();

        let start = root.join("sub/src");
        std::fs::create_dir_all(&start).unwrap();

        assert_eq!(discover_project_root(&start), root.join("sub"));
    }

    #[test]
    fn discover_project_root_falls_back_to_git_when_no_promptpack() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("sub/src")).unwrap();

        let start = root.join("sub/src");
        assert_eq!(discover_project_root(&start), root.to_path_buf());
    }

    #[test]
    fn discover_project_root_uses_lockfile_when_present() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::create_dir_all(root.join("sub/.promptpack")).unwrap();
        std::fs::write(root.join("sub/calvin.lock"), "").unwrap();

        let start = root.join("sub/src");
        std::fs::create_dir_all(&start).unwrap();

        assert_eq!(discover_project_root(&start), root.join("sub"));
    }
}
