//! Terminal rendering functions for tree menu.
//!
//! This module provides functions to render tree nodes, status bars,
//! and help text to strings for terminal output.

use crate::ui::theme::{icons, icons_ascii};

use super::menu::FlattenedNode;
use super::node::SelectionState;

/// Render a single tree node to a string
pub fn render_tree_node(node: &FlattenedNode, is_active: bool, supports_unicode: bool) -> String {
    let indent = "  ".repeat(node.depth);
    let cursor = if is_active { "> " } else { "  " };

    // Selection state icon
    let state_icon = match node.state {
        SelectionState::Selected => {
            if supports_unicode {
                icons::SELECTED
            } else {
                icons_ascii::SELECTED
            }
        }
        SelectionState::Unselected => {
            if supports_unicode {
                icons::UNSELECTED
            } else {
                icons_ascii::UNSELECTED
            }
        }
        SelectionState::Partial => {
            if supports_unicode {
                icons::PARTIAL
            } else {
                icons_ascii::PARTIAL
            }
        }
    };

    // Expansion icon (only for nodes with children)
    let expand_icon = if node.has_children {
        if node.expanded {
            if supports_unicode {
                format!("{} ", icons::EXPAND)
            } else {
                format!("{} ", icons_ascii::EXPAND)
            }
        } else if supports_unicode {
            format!("{} ", icons::COLLAPSE)
        } else {
            format!("{} ", icons_ascii::COLLAPSE)
        }
    } else {
        String::from("  ")
    };

    // File count suffix
    let count_suffix = if node.has_children && node.file_count > 0 {
        format!(" ({} files)", node.file_count)
    } else {
        String::new()
    };

    format!(
        "{}{}{}{} {}{}",
        cursor, indent, expand_icon, state_icon, node.label, count_suffix
    )
}

/// Render the status bar showing selection counts
pub fn render_status_bar(selected: usize, total: usize, supports_unicode: bool) -> String {
    let selected_icon = if supports_unicode {
        icons::SELECTED
    } else {
        icons_ascii::SELECTED
    };
    let unselected_icon = if supports_unicode {
        icons::UNSELECTED
    } else {
        icons_ascii::UNSELECTED
    };
    let partial_icon = if supports_unicode {
        icons::PARTIAL
    } else {
        icons_ascii::PARTIAL
    };

    format!(
        "Selected: {}/{} files\n\n{} = selected    {} = partial    {} = not selected",
        selected, total, selected_icon, partial_icon, unselected_icon
    )
}

/// Render the help bar showing keyboard shortcuts
pub fn render_help_bar() -> String {
    String::from(
        "[a] All    [n] None    [i] Invert    [Enter] Confirm    [q] Quit\n\
         (Use ↑↓ to navigate, Space to toggle, →← to expand/collapse)",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets::tree_menu::menu::TreeMenu;
    use crate::ui::widgets::tree_menu::node::TreeNode;
    use std::path::PathBuf;

    #[test]
    fn render_shows_selection_icons() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        root.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        root.children[0].select();

        let menu = TreeMenu::new(root);
        let rendered = menu.render(true);

        assert!(rendered.contains("●"), "Should contain selected icon");
    }

    #[test]
    fn render_shows_cursor_indicator() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        let mut home = TreeNode::new("Home");
        home.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("~/.claude/a.md"),
            "home:~/.claude/a.md".to_string(),
        ));
        let mut project = TreeNode::new("Project");
        project.add_child(TreeNode::leaf(
            "c.md",
            PathBuf::from(".cursor/c.md"),
            "project:.cursor/c.md".to_string(),
        ));
        root.add_child(home);
        root.add_child(project);

        let menu = TreeMenu::new(root);
        let rendered = menu.render(true);

        // First line should have cursor indicator
        let first_line = rendered.lines().next().unwrap();
        assert!(
            first_line.starts_with("> "),
            "First line should have cursor"
        );
    }

    #[test]
    fn render_shows_expansion_icons() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        let mut home = TreeNode::new("Home");
        home.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("~/.claude/a.md"),
            "home:~/.claude/a.md".to_string(),
        ));
        let mut project = TreeNode::new("Project");
        project.add_child(TreeNode::leaf(
            "c.md",
            PathBuf::from(".cursor/c.md"),
            "project:.cursor/c.md".to_string(),
        ));
        root.add_child(home);
        root.add_child(project);

        let mut menu = TreeMenu::new(root);

        // Expand Home
        menu.handle_action(super::super::menu::TreeAction::Down);
        menu.handle_action(super::super::menu::TreeAction::Expand);

        let rendered = menu.render(true);
        assert!(rendered.contains("▼"), "Should show expanded icon");
    }

    #[test]
    fn render_status_bar_shows_counts() {
        let status = render_status_bar(3, 3, true);
        assert!(status.contains("3/3"), "Should show 3/3 selected");
    }

    #[test]
    fn render_help_bar_shows_shortcuts() {
        let help = render_help_bar();
        assert!(help.contains("[a] All"));
        assert!(help.contains("[n] None"));
        assert!(help.contains("[i] Invert"));
        assert!(help.contains("[Enter] Confirm"));
        assert!(help.contains("[q] Quit"));
    }

    #[test]
    fn render_ascii_fallback() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        root.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        root.children[0].select();

        let menu = TreeMenu::new(root);
        let rendered = menu.render(false); // ASCII mode

        assert!(
            rendered.contains("[x]"),
            "Should contain ASCII selected icon"
        );
    }

    #[test]
    fn render_partial_icon_unicode() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        root.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "a".to_string(),
        ));
        root.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "b".to_string(),
        ));
        root.children[0].select();
        root.update_state_from_children();

        let menu = TreeMenu::new(root);
        let rendered = menu.render(true);

        assert!(rendered.contains("◐"), "Should contain partial icon");
    }

    #[test]
    fn render_partial_icon_ascii() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        root.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "a".to_string(),
        ));
        root.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "b".to_string(),
        ));
        root.children[0].select();
        root.update_state_from_children();

        let menu = TreeMenu::new(root);
        let rendered = menu.render(false);

        // ASCII partial icon is [-] per theme.rs
        assert!(
            rendered.contains("[-]"),
            "Should contain ASCII partial icon [-]"
        );
    }

    #[test]
    fn render_deep_indentation() {
        // Create 5-level tree
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        let mut l1 = TreeNode::new("L1");
        l1.expanded = true;
        let mut l2 = TreeNode::new("L2");
        l2.expanded = true;
        let mut l3 = TreeNode::new("L3");
        l3.expanded = true;
        let mut l4 = TreeNode::new("L4");
        l4.expanded = true;
        l4.add_child(TreeNode::leaf(
            "deep.md",
            PathBuf::from("deep.md"),
            "d".to_string(),
        ));
        l3.add_child(l4);
        l2.add_child(l3);
        l1.add_child(l2);
        root.add_child(l1);

        let menu = TreeMenu::new(root);
        let rendered = menu.render(true);

        // The deepest node should have 5 levels of indentation (10 spaces)
        let deep_line = rendered.lines().last().unwrap();
        assert!(
            deep_line.contains("          "),
            "Deep node should have 5 levels of indentation: '{}'",
            deep_line
        );
    }

    #[test]
    fn render_long_label_not_truncated() {
        let long_label = "this-is-a-very-long-file-name-that-should-not-be-truncated.md";
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        root.add_child(TreeNode::leaf(
            long_label,
            PathBuf::from(long_label),
            "key".to_string(),
        ));

        let menu = TreeMenu::new(root);
        let rendered = menu.render(true);

        assert!(
            rendered.contains(long_label),
            "Long label should appear in full"
        );
    }

    #[test]
    fn render_status_bar_zero_selected() {
        let status = render_status_bar(0, 3, true);
        assert!(status.contains("0/3"), "Should show 0/3 selected");
    }

    #[test]
    fn render_collapsed_vs_expanded_icon() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        let mut child = TreeNode::new("Child");
        child.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "a".to_string(),
        ));
        root.add_child(child);

        let mut menu = TreeMenu::new(root);

        // Child is collapsed - should show ▶
        let rendered_collapsed = menu.render(true);
        assert!(
            rendered_collapsed.contains("▶"),
            "Collapsed should show ▶: {}",
            rendered_collapsed
        );

        // Expand child
        menu.handle_action(super::super::menu::TreeAction::Down);
        menu.handle_action(super::super::menu::TreeAction::Expand);

        let rendered_expanded = menu.render(true);
        assert!(
            rendered_expanded.contains("▼"),
            "Expanded should show ▼: {}",
            rendered_expanded
        );
    }
}
