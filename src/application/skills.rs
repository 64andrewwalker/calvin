//! Skills application helpers
//!
//! Shared helpers used by multiple use cases when reasoning about directory-based skill outputs.

use std::path::{Path, PathBuf};

pub(crate) fn skill_root_from_path(path: &Path) -> Option<PathBuf> {
    for ancestor in path.ancestors() {
        let parent = ancestor.parent()?;
        if parent.file_name()? != std::ffi::OsStr::new("skills") {
            continue;
        }
        let grandparent = parent.parent()?;
        let Some(name) = grandparent.file_name() else {
            continue;
        };
        if name == std::ffi::OsStr::new(".claude") || name == std::ffi::OsStr::new(".codex") {
            return Some(ancestor.to_path_buf());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_root_from_path_detects_codex_skill_root_from_skill_md() {
        let path = PathBuf::from(".codex/skills/my-skill/SKILL.md");
        assert_eq!(
            skill_root_from_path(&path),
            Some(PathBuf::from(".codex/skills/my-skill"))
        );
    }

    #[test]
    fn skill_root_from_path_detects_claude_skill_root_from_nested_file() {
        let path = PathBuf::from(".claude/skills/my-skill/scripts/validate.py");
        assert_eq!(
            skill_root_from_path(&path),
            Some(PathBuf::from(".claude/skills/my-skill"))
        );
    }

    #[test]
    fn skill_root_from_path_returns_none_outside_skills() {
        let path = PathBuf::from(".claude/commands/test.md");
        assert_eq!(skill_root_from_path(&path), None);
    }

    #[test]
    #[allow(non_snake_case)] // naming convention: `<original_test_name>__<variant_type>`
    fn skill_root_from_path_returns_none_outside_skills__when_path_is_skills_dir() {
        let path = PathBuf::from(".codex/skills");
        assert_eq!(skill_root_from_path(&path), None);
    }
}
