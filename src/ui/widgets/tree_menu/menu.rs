//! TreeMenu state management and action handling.
//!
//! This module contains the main TreeMenu component that manages
//! the tree state, handles user actions, and coordinates rendering.

use std::path::PathBuf;

use super::node::{SelectionState, TreeNode};
use super::render::{render_help_bar, render_status_bar, render_tree_node};

/// A flattened representation of a tree node for rendering
#[derive(Debug, Clone)]
pub struct FlattenedNode {
    /// Index in the original tree structure (for updates)
    pub path: Vec<usize>,
    /// Depth level (0 = root)
    pub depth: usize,
    /// Display label
    pub label: String,
    /// Selection state
    pub state: SelectionState,
    /// Whether this node is expanded (for non-leaves)
    pub expanded: bool,
    /// Whether this node has children
    pub has_children: bool,
    /// File count for this subtree
    pub file_count: usize,
}

/// Tree menu action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeAction {
    /// Move cursor up
    Up,
    /// Move cursor down
    Down,
    /// Toggle selection
    Toggle,
    /// Expand node
    Expand,
    /// Collapse node
    Collapse,
    /// Select all
    SelectAll,
    /// Select none
    SelectNone,
    /// Invert selection
    Invert,
    /// Confirm selection
    Confirm,
    /// Quit without confirming
    Quit,
}

/// Interactive tree menu for selecting items
pub struct TreeMenu {
    /// Root node containing all items
    pub root: TreeNode,
    /// Current cursor position in flattened view
    pub cursor: usize,
    /// Cached flattened nodes for rendering
    flattened: Vec<FlattenedNode>,
}

impl TreeMenu {
    /// Create a new tree menu from a root node
    pub fn new(root: TreeNode) -> Self {
        let mut menu = Self {
            root,
            cursor: 0,
            flattened: Vec::new(),
        };
        menu.rebuild_flattened();
        menu
    }

    /// Rebuild the flattened node list
    pub fn rebuild_flattened(&mut self) {
        self.flattened = Self::flatten_node(&self.root, 0, &[]);
        // Ensure cursor is within bounds
        if !self.flattened.is_empty() && self.cursor >= self.flattened.len() {
            self.cursor = self.flattened.len() - 1;
        }
    }

    fn flatten_node(node: &TreeNode, depth: usize, path: &[usize]) -> Vec<FlattenedNode> {
        let mut result = Vec::new();

        // Add the current node
        result.push(FlattenedNode {
            path: path.to_vec(),
            depth,
            label: node.label.clone(),
            state: node.state,
            expanded: node.expanded,
            has_children: !node.children.is_empty(),
            file_count: node.total_count(),
        });

        // If expanded, add children
        if node.expanded {
            for (i, child) in node.children.iter().enumerate() {
                let mut child_path = path.to_vec();
                child_path.push(i);
                result.extend(Self::flatten_node(child, depth + 1, &child_path));
            }
        }

        result
    }

    /// Get flattened nodes for rendering
    pub fn flattened_nodes(&self) -> &[FlattenedNode] {
        &self.flattened
    }

    /// Get current cursor position
    pub fn cursor_position(&self) -> usize {
        self.cursor
    }

    /// Handle a tree action
    pub fn handle_action(&mut self, action: TreeAction) -> bool {
        match action {
            TreeAction::Up => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                false
            }
            TreeAction::Down => {
                if self.cursor + 1 < self.flattened.len() {
                    self.cursor += 1;
                }
                false
            }
            TreeAction::Toggle => {
                if let Some(node) = self.get_node_mut() {
                    node.toggle();
                }
                self.propagate_state_changes();
                self.rebuild_flattened();
                false
            }
            TreeAction::Expand => {
                if let Some(node) = self.get_node_mut() {
                    if !node.children.is_empty() {
                        node.expand();
                    }
                }
                self.rebuild_flattened();
                false
            }
            TreeAction::Collapse => {
                if let Some(node) = self.get_node_mut() {
                    node.collapse();
                }
                self.rebuild_flattened();
                false
            }
            TreeAction::SelectAll => {
                self.root.select_all();
                self.rebuild_flattened();
                false
            }
            TreeAction::SelectNone => {
                self.root.select_none();
                self.rebuild_flattened();
                false
            }
            TreeAction::Invert => {
                self.root.invert();
                self.rebuild_flattened();
                false
            }
            TreeAction::Confirm => true,
            TreeAction::Quit => true,
        }
    }

    /// Get mutable reference to the current node
    fn get_node_mut(&mut self) -> Option<&mut TreeNode> {
        if self.flattened.is_empty() {
            return None;
        }
        let path = self.flattened.get(self.cursor)?.path.clone();
        Self::get_node_at_path(&mut self.root, &path)
    }

    fn get_node_at_path<'a>(node: &'a mut TreeNode, path: &[usize]) -> Option<&'a mut TreeNode> {
        if path.is_empty() {
            return Some(node);
        }
        let idx = path[0];
        if idx >= node.children.len() {
            return None;
        }
        Self::get_node_at_path(&mut node.children[idx], &path[1..])
    }

    /// Propagate state changes up the tree
    fn propagate_state_changes(&mut self) {
        Self::update_parent_states(&mut self.root);
    }

    fn update_parent_states(node: &mut TreeNode) {
        for child in &mut node.children {
            Self::update_parent_states(child);
        }
        node.update_state_from_children();
    }

    /// Get selected paths
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.root.selected_paths()
    }

    /// Get selected keys
    pub fn selected_keys(&self) -> Vec<String> {
        self.root.selected_keys()
    }

    /// Get selected count
    pub fn selected_count(&self) -> usize {
        self.root.selected_count()
    }

    /// Get total count
    pub fn total_count(&self) -> usize {
        self.root.total_count()
    }

    /// Render the tree menu to a string
    pub fn render(&self, supports_unicode: bool) -> String {
        let mut out = String::new();

        for (i, node) in self.flattened.iter().enumerate() {
            let is_active = i == self.cursor;
            let line = render_tree_node(node, is_active, supports_unicode);
            out.push_str(&line);
            out.push('\n');
        }

        out
    }

    /// Render the status bar
    pub fn render_status_bar(&self, supports_unicode: bool) -> String {
        let selected = self.selected_count();
        let total = self.total_count();
        render_status_bar(selected, total, supports_unicode)
    }

    /// Render the help bar
    pub fn render_help_bar(&self) -> String {
        render_help_bar()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tree() -> TreeNode {
        let mut root = TreeNode::new("Root");
        root.expanded = true;

        let mut home = TreeNode::new("Home");
        home.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("~/.claude/a.md"),
            "home:~/.claude/a.md".to_string(),
        ));
        home.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("~/.claude/b.md"),
            "home:~/.claude/b.md".to_string(),
        ));

        let mut project = TreeNode::new("Project");
        project.add_child(TreeNode::leaf(
            "c.md",
            PathBuf::from(".cursor/c.md"),
            "project:.cursor/c.md".to_string(),
        ));

        root.add_child(home);
        root.add_child(project);
        root
    }

    #[test]
    fn tree_menu_flattens_visible_nodes() {
        let root = create_test_tree();
        let menu = TreeMenu::new(root);

        // Root is expanded, so we see Root + 2 children (Home, Project)
        // Home and Project are collapsed
        assert_eq!(menu.flattened_nodes().len(), 3);
    }

    #[test]
    fn tree_menu_expands_node() {
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);

        // Move to Home (index 1) and expand
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Expand);

        // Now Home's children are visible
        // Root, Home, a.md, b.md, Project
        assert_eq!(menu.flattened_nodes().len(), 5);
    }

    #[test]
    fn tree_menu_cursor_navigation() {
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);

        assert_eq!(menu.cursor_position(), 0);
        menu.handle_action(TreeAction::Down);
        assert_eq!(menu.cursor_position(), 1);
        menu.handle_action(TreeAction::Down);
        assert_eq!(menu.cursor_position(), 2);
        menu.handle_action(TreeAction::Up);
        assert_eq!(menu.cursor_position(), 1);
    }

    #[test]
    fn tree_menu_cursor_bounds() {
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);

        // Try to go up at top
        menu.handle_action(TreeAction::Up);
        assert_eq!(menu.cursor_position(), 0);

        // Go to bottom
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Down); // Try past end
        assert_eq!(menu.cursor_position(), 2);
    }

    #[test]
    fn tree_menu_toggle_selection() {
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);

        // Expand Home to see children
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Expand);

        // Move to a.md and toggle
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Toggle);

        assert_eq!(menu.selected_count(), 1);
    }

    #[test]
    fn tree_menu_select_all() {
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);

        menu.handle_action(TreeAction::SelectAll);

        assert_eq!(menu.selected_count(), 3); // a.md, b.md, c.md
    }

    #[test]
    fn tree_menu_select_none() {
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);

        menu.handle_action(TreeAction::SelectAll);
        menu.handle_action(TreeAction::SelectNone);

        assert_eq!(menu.selected_count(), 0);
    }

    #[test]
    fn tree_menu_invert() {
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);

        // Expand Home
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Expand);

        // Select a.md
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Toggle);
        assert_eq!(menu.selected_count(), 1);

        // Invert
        menu.handle_action(TreeAction::Invert);
        assert_eq!(menu.selected_count(), 2); // b.md and c.md now selected
    }

    // === Edge Case Tests ===

    #[test]
    fn menu_empty_tree() {
        let root = TreeNode::new("Empty");
        let menu = TreeMenu::new(root);

        assert_eq!(menu.flattened_nodes().len(), 1); // Just root
                                                     // A non-leaf node with no children still counts as 1 in total_count
        assert_eq!(menu.total_count(), 1);
        assert_eq!(menu.selected_count(), 0);
    }

    #[test]
    fn menu_cursor_stays_valid_after_collapse() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        let mut child = TreeNode::new("Child");
        child.expanded = true;
        child.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "a".to_string(),
        ));
        child.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "b".to_string(),
        ));
        root.add_child(child);

        let mut menu = TreeMenu::new(root);

        // Move cursor to b.md (index 3: Root, Child, a.md, b.md)
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Down);
        assert_eq!(menu.cursor_position(), 3);

        // Now collapse Child (at index 1)
        menu.cursor = 1;
        menu.handle_action(TreeAction::Collapse);

        // Cursor should be clamped to valid range
        assert!(menu.cursor_position() < menu.flattened_nodes().len());
    }

    #[test]
    fn menu_toggle_on_collapsed_parent_selects_hidden_children() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        let mut child = TreeNode::new("Child");
        // Child is collapsed
        child.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "a".to_string(),
        ));
        child.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "b".to_string(),
        ));
        root.add_child(child);

        let mut menu = TreeMenu::new(root);

        // Move to Child and toggle
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Toggle);

        // Both hidden children should be selected
        assert_eq!(menu.selected_count(), 2);
    }

    #[test]
    fn menu_expand_leaf_no_effect() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        root.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "a".to_string(),
        ));

        let mut menu = TreeMenu::new(root);

        // Move to leaf
        menu.handle_action(TreeAction::Down);

        // Try to expand - should have no effect
        let flattened_before = menu.flattened_nodes().len();
        menu.handle_action(TreeAction::Expand);
        let flattened_after = menu.flattened_nodes().len();

        assert_eq!(flattened_before, flattened_after);
    }

    #[test]
    fn menu_collapse_already_collapsed_no_effect() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        let child = TreeNode::new("Child"); // Already collapsed
        root.add_child(child);

        let mut menu = TreeMenu::new(root);

        // Move to child
        menu.handle_action(TreeAction::Down);

        // Try to collapse - should have no effect
        let flattened_before = menu.flattened_nodes().len();
        menu.handle_action(TreeAction::Collapse);
        let flattened_after = menu.flattened_nodes().len();

        assert_eq!(flattened_before, flattened_after);
    }

    #[test]
    fn menu_selected_keys_after_bulk_operations() {
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);

        // SelectAll then SelectNone should result in empty
        menu.handle_action(TreeAction::SelectAll);
        assert_eq!(menu.selected_count(), 3);

        menu.handle_action(TreeAction::SelectNone);
        assert_eq!(menu.selected_count(), 0);
        assert!(menu.selected_keys().is_empty());
    }

    #[test]
    fn menu_flattened_order_matches_tree_order() {
        let mut root = TreeNode::new("Root");
        root.expanded = true;
        let mut home = TreeNode::new("Home");
        home.expanded = true;
        home.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "a".to_string(),
        ));
        home.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "b".to_string(),
        ));
        root.add_child(home);

        let menu = TreeMenu::new(root);
        let nodes = menu.flattened_nodes();

        // DFS order: Root, Home, a.md, b.md
        assert_eq!(nodes[0].label, "Root");
        assert_eq!(nodes[1].label, "Home");
        assert_eq!(nodes[2].label, "a.md");
        assert_eq!(nodes[3].label, "b.md");
    }
}
