//! Tree construction from lockfile entries.
//!
//! This module provides functions to build a tree structure from
//! lockfile entries, grouping by scope and target.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::node::TreeNode;

/// Build a tree from lockfile entries
///
/// Groups entries by Scope → Target (inferred from path).
/// Since the lockfile doesn't store target info, we infer from the path prefix.
pub fn build_tree_from_lockfile(entries: impl IntoIterator<Item = (String, PathBuf)>) -> TreeNode {
    let mut root = TreeNode::new("Deployments");
    root.expanded = true;

    // Group by scope and target
    let mut home_targets: HashMap<String, Vec<(String, PathBuf)>> = HashMap::new();
    let mut project_targets: HashMap<String, Vec<(String, PathBuf)>> = HashMap::new();

    for (key, path) in entries {
        // Infer target from path
        let target = infer_target_from_path(&path);

        if key.starts_with("home:") {
            home_targets.entry(target).or_default().push((key, path));
        } else if key.starts_with("project:") {
            project_targets.entry(target).or_default().push((key, path));
        }
    }

    // Build Home scope
    if !home_targets.is_empty() {
        let mut home_node = TreeNode::new("Home (~/)".to_string());

        // Sort targets alphabetically
        let mut targets: Vec<_> = home_targets.into_iter().collect();
        targets.sort_by(|a, b| a.0.cmp(&b.0));

        for (target, files) in targets {
            let mut target_node = TreeNode::new(target);

            for (key, path) in files {
                let label = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                target_node.add_child(TreeNode::leaf(label, path, key));
            }

            home_node.add_child(target_node);
        }

        root.add_child(home_node);
    }

    // Build Project scope
    if !project_targets.is_empty() {
        let mut project_node = TreeNode::new("Project (./)".to_string());

        // Sort targets alphabetically
        let mut targets: Vec<_> = project_targets.into_iter().collect();
        targets.sort_by(|a, b| a.0.cmp(&b.0));

        for (target, files) in targets {
            let mut target_node = TreeNode::new(target);

            for (key, path) in files {
                let label = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.display().to_string());
                target_node.add_child(TreeNode::leaf(label, path, key));
            }

            project_node.add_child(target_node);
        }

        root.add_child(project_node);
    }

    root
}

/// Infer target platform from file path
pub fn infer_target_from_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    if path_str.contains(".claude/") || path_str.contains(".claude\\") {
        "claude-code".to_string()
    } else if path_str.contains(".cursor/") || path_str.contains(".cursor\\") {
        "cursor".to_string()
    } else if path_str.contains(".vscode/") || path_str.contains(".vscode\\") {
        "vscode".to_string()
    } else if path_str.contains(".codex/") || path_str.contains(".codex\\") {
        "codex".to_string()
    } else if path_str.contains(".gemini/") || path_str.contains(".gemini\\") {
        "antigravity".to_string()
    } else if path_str.contains("AGENTS.md")
        || path_str.contains("CLAUDE.md")
        || path_str.contains("GEMINI.md")
    {
        "agents-md".to_string()
    } else {
        "other".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_tree_from_lockfile_works() {
        let entries = vec![
            (
                "home:~/.claude/a.md".to_string(),
                PathBuf::from("~/.claude/a.md"),
            ),
            (
                "project:.cursor/b.md".to_string(),
                PathBuf::from(".cursor/b.md"),
            ),
        ];

        let root = build_tree_from_lockfile(entries);

        assert_eq!(root.children.len(), 2);
        assert_eq!(root.total_count(), 2);
    }

    #[test]
    fn builder_empty_entries() {
        let entries: Vec<(String, PathBuf)> = vec![];
        let root = build_tree_from_lockfile(entries);

        assert_eq!(root.label, "Deployments");
        assert!(root.children.is_empty());
        // Root with no children is considered a leaf with count 1
        assert_eq!(root.total_count(), 1);
    }

    #[test]
    fn builder_home_only() {
        let entries = vec![(
            "home:~/.claude/test.md".to_string(),
            PathBuf::from("/Users/test/.claude/test.md"),
        )];

        let root = build_tree_from_lockfile(entries);

        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].label, "Home (~/)");
    }

    #[test]
    fn builder_project_only() {
        let entries = vec![(
            "project:.cursor/test.md".to_string(),
            PathBuf::from(".cursor/test.md"),
        )];

        let root = build_tree_from_lockfile(entries);

        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].label, "Project (./)");
    }

    #[test]
    fn builder_unknown_target_path() {
        let entries = vec![(
            "home:~/unknown/test.md".to_string(),
            PathBuf::from("/Users/test/unknown/test.md"),
        )];

        let root = build_tree_from_lockfile(entries);

        // Should have Home -> other -> test.md
        assert_eq!(root.children.len(), 1);
        assert_eq!(root.children[0].children[0].label, "other");
    }

    #[test]
    fn builder_infer_all_targets() {
        assert_eq!(
            infer_target_from_path(&PathBuf::from("/Users/test/.claude/commands/test.md")),
            "claude-code"
        );
        assert_eq!(
            infer_target_from_path(&PathBuf::from("/Users/test/.cursor/rules/test.mdc")),
            "cursor"
        );
        assert_eq!(
            infer_target_from_path(&PathBuf::from("/Users/test/.vscode/instructions/test.md")),
            "vscode"
        );
        assert_eq!(
            infer_target_from_path(&PathBuf::from("/Users/test/.codex/prompts/test.md")),
            "codex"
        );
        assert_eq!(
            infer_target_from_path(&PathBuf::from(
                "/Users/test/.gemini/antigravity/workflows/test.md"
            )),
            "antigravity"
        );
        assert_eq!(
            infer_target_from_path(&PathBuf::from("/project/AGENTS.md")),
            "agents-md"
        );
        assert_eq!(
            infer_target_from_path(&PathBuf::from("/project/CLAUDE.md")),
            "agents-md"
        );
        assert_eq!(
            infer_target_from_path(&PathBuf::from("/project/GEMINI.md")),
            "agents-md"
        );
        assert_eq!(
            infer_target_from_path(&PathBuf::from("/Users/test/random/file.md")),
            "other"
        );
    }

    #[test]
    fn builder_mixed_targets_sorted() {
        let entries = vec![
            (
                "home:~/.vscode/a.md".to_string(),
                PathBuf::from("/Users/test/.vscode/a.md"),
            ),
            (
                "home:~/.claude/b.md".to_string(),
                PathBuf::from("/Users/test/.claude/b.md"),
            ),
            (
                "home:~/.cursor/c.md".to_string(),
                PathBuf::from("/Users/test/.cursor/c.md"),
            ),
        ];

        let root = build_tree_from_lockfile(entries);

        // Home should have 3 target children, sorted alphabetically
        let home = &root.children[0];
        assert_eq!(home.children[0].label, "claude-code");
        assert_eq!(home.children[1].label, "cursor");
        assert_eq!(home.children[2].label, "vscode");
    }

    #[test]
    fn render_tree_structure_with_real_entries() {
        use super::super::menu::TreeMenu;

        // Create entries with various targets for each scope
        let entries = vec![
            (
                "home:~/.claude/commands/test1.md".to_string(),
                PathBuf::from("/Users/test/.claude/commands/test1.md"),
            ),
            (
                "home:~/.claude/commands/test2.md".to_string(),
                PathBuf::from("/Users/test/.claude/commands/test2.md"),
            ),
            (
                "home:~/.cursor/rules/prompt1.mdc".to_string(),
                PathBuf::from("/Users/test/.cursor/rules/prompt1.mdc"),
            ),
            (
                "home:~/.vscode/instructions/task.instructions.md".to_string(),
                PathBuf::from("/Users/test/.vscode/instructions/task.instructions.md"),
            ),
            (
                "home:~/.codex/prompts/help.md".to_string(),
                PathBuf::from("/Users/test/.codex/prompts/help.md"),
            ),
            (
                "home:~/.gemini/antigravity/global_workflows/flow.md".to_string(),
                PathBuf::from("/Users/test/.gemini/antigravity/global_workflows/flow.md"),
            ),
            (
                "project:.cursor/rules/local.mdc".to_string(),
                PathBuf::from(".cursor/rules/local.mdc"),
            ),
        ];

        let root = build_tree_from_lockfile(entries);
        let menu = TreeMenu::new(root);

        // Verify tree structure
        assert_eq!(menu.total_count(), 7);
    }
}
