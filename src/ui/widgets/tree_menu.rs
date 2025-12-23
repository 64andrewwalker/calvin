//! Tree Menu Widget
//!
//! A hierarchical tree structure for selecting items in the clean command.
//! Supports parent-child selection propagation and partial selection states.

#![allow(dead_code)] // Phase 2: Tree menu not yet integrated into clean command

use std::path::PathBuf;

/// Selection state for a tree node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionState {
    /// All items selected (●)
    Selected,
    /// No items selected (○)
    Unselected,
    /// Some but not all items selected (◐)
    Partial,
}

/// A node in the tree structure
#[derive(Debug, Clone)]
pub struct TreeNode {
    /// Display label for this node
    pub label: String,
    /// Current selection state
    pub state: SelectionState,
    /// Child nodes (empty for leaf nodes)
    pub children: Vec<TreeNode>,
    /// Whether the node is expanded (for non-leaf nodes)
    pub expanded: bool,
    /// Associated file path (for leaf nodes)
    pub path: Option<PathBuf>,
    /// Lockfile key (for leaf nodes)
    pub key: Option<String>,
}

impl TreeNode {
    /// Create a new tree node
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            state: SelectionState::Unselected,
            children: Vec::new(),
            expanded: false,
            path: None,
            key: None,
        }
    }

    /// Create a leaf node with path
    pub fn leaf(label: impl Into<String>, path: PathBuf, key: String) -> Self {
        Self {
            label: label.into(),
            state: SelectionState::Unselected,
            children: Vec::new(),
            expanded: false,
            path: Some(path),
            key: Some(key),
        }
    }

    /// Check if this is a leaf node
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Add a child node
    pub fn add_child(&mut self, child: TreeNode) {
        self.children.push(child);
    }

    /// Select this node and all children
    pub fn select(&mut self) {
        self.state = SelectionState::Selected;
        for child in &mut self.children {
            child.select();
        }
    }

    /// Deselect this node and all children
    pub fn deselect(&mut self) {
        self.state = SelectionState::Unselected;
        for child in &mut self.children {
            child.deselect();
        }
    }

    /// Toggle selection state
    pub fn toggle(&mut self) {
        match self.state {
            SelectionState::Selected => self.deselect(),
            SelectionState::Unselected | SelectionState::Partial => self.select(),
        }
    }

    /// Update parent state based on children states
    /// Returns true if state changed
    pub fn update_state_from_children(&mut self) -> bool {
        if self.children.is_empty() {
            return false;
        }

        let old_state = self.state;
        let all_selected = self
            .children
            .iter()
            .all(|c| c.state == SelectionState::Selected);
        let all_unselected = self
            .children
            .iter()
            .all(|c| c.state == SelectionState::Unselected);

        self.state = if all_selected {
            SelectionState::Selected
        } else if all_unselected {
            SelectionState::Unselected
        } else {
            SelectionState::Partial
        };

        old_state != self.state
    }

    /// Get count of selected leaf nodes
    pub fn selected_count(&self) -> usize {
        if self.is_leaf() {
            if self.state == SelectionState::Selected {
                1
            } else {
                0
            }
        } else {
            self.children.iter().map(|c| c.selected_count()).sum()
        }
    }

    /// Get count of all leaf nodes
    pub fn total_count(&self) -> usize {
        if self.is_leaf() {
            1
        } else {
            self.children.iter().map(|c| c.total_count()).sum()
        }
    }

    /// Get all selected leaf paths
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        if self.is_leaf() {
            if self.state == SelectionState::Selected {
                self.path.clone().into_iter().collect()
            } else {
                Vec::new()
            }
        } else {
            self.children
                .iter()
                .flat_map(|c| c.selected_paths())
                .collect()
        }
    }

    /// Get all selected keys
    pub fn selected_keys(&self) -> Vec<String> {
        if self.is_leaf() {
            if self.state == SelectionState::Selected {
                self.key.clone().into_iter().collect()
            } else {
                Vec::new()
            }
        } else {
            self.children
                .iter()
                .flat_map(|c| c.selected_keys())
                .collect()
        }
    }

    /// Expand this node
    pub fn expand(&mut self) {
        self.expanded = true;
    }

    /// Collapse this node
    pub fn collapse(&mut self) {
        self.expanded = false;
    }

    /// Toggle expansion
    pub fn toggle_expand(&mut self) {
        self.expanded = !self.expanded;
    }

    /// Select all nodes
    pub fn select_all(&mut self) {
        self.select();
    }

    /// Deselect all nodes
    pub fn select_none(&mut self) {
        self.deselect();
    }

    /// Invert selection
    pub fn invert(&mut self) {
        if self.is_leaf() {
            match self.state {
                SelectionState::Selected => self.state = SelectionState::Unselected,
                SelectionState::Unselected => self.state = SelectionState::Selected,
                SelectionState::Partial => {} // Shouldn't happen for leaf nodes
            }
        } else {
            for child in &mut self.children {
                child.invert();
            }
            self.update_state_from_children();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === TDD: Phase 2.1 - TreeNode Structure ===

    #[test]
    fn tree_node_new_is_unselected() {
        let node = TreeNode::new("Home");
        assert_eq!(node.state, SelectionState::Unselected);
        assert!(!node.expanded);
        assert!(node.children.is_empty());
    }

    #[test]
    fn tree_node_leaf_has_path() {
        let node = TreeNode::leaf(
            "test.md",
            PathBuf::from("~/.claude/commands/test.md"),
            "home:~/.claude/commands/test.md".to_string(),
        );
        assert!(node.is_leaf());
        assert!(node.path.is_some());
        assert!(node.key.is_some());
    }

    #[test]
    fn tree_node_add_child() {
        let mut parent = TreeNode::new("Home");
        parent.add_child(TreeNode::new("claude-code"));
        assert_eq!(parent.children.len(), 1);
        assert!(!parent.is_leaf());
    }

    // === TDD: Phase 2.1 - Selection ===

    #[test]
    fn selecting_node_sets_selected() {
        let mut node = TreeNode::new("Home");
        node.select();
        assert_eq!(node.state, SelectionState::Selected);
    }

    #[test]
    fn selecting_parent_selects_all_children() {
        let mut parent = TreeNode::new("Home");
        parent.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        parent.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));

        parent.select();

        assert_eq!(parent.state, SelectionState::Selected);
        assert!(parent
            .children
            .iter()
            .all(|c| c.state == SelectionState::Selected));
    }

    #[test]
    fn deselecting_parent_deselects_all_children() {
        let mut parent = TreeNode::new("Home");
        parent.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        parent.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));
        parent.select();

        parent.deselect();

        assert_eq!(parent.state, SelectionState::Unselected);
        assert!(parent
            .children
            .iter()
            .all(|c| c.state == SelectionState::Unselected));
    }

    #[test]
    fn toggle_unselected_selects() {
        let mut node = TreeNode::new("Home");
        node.toggle();
        assert_eq!(node.state, SelectionState::Selected);
    }

    #[test]
    fn toggle_selected_deselects() {
        let mut node = TreeNode::new("Home");
        node.select();
        node.toggle();
        assert_eq!(node.state, SelectionState::Unselected);
    }

    #[test]
    fn toggle_partial_selects() {
        let mut node = TreeNode::new("Home");
        node.state = SelectionState::Partial;
        node.toggle();
        assert_eq!(node.state, SelectionState::Selected);
    }

    // === TDD: Phase 2.1 - Partial Selection ===

    #[test]
    fn partial_child_selection_shows_partial_parent() {
        let mut parent = TreeNode::new("Home");
        parent.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        parent.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));

        // Select only first child
        parent.children[0].select();
        parent.update_state_from_children();

        assert_eq!(parent.state, SelectionState::Partial);
    }

    #[test]
    fn all_children_selected_shows_selected_parent() {
        let mut parent = TreeNode::new("Home");
        parent.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        parent.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));

        parent.children[0].select();
        parent.children[1].select();
        parent.update_state_from_children();

        assert_eq!(parent.state, SelectionState::Selected);
    }

    #[test]
    fn no_children_selected_shows_unselected_parent() {
        let mut parent = TreeNode::new("Home");
        parent.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        parent.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));

        parent.update_state_from_children();

        assert_eq!(parent.state, SelectionState::Unselected);
    }

    // === TDD: Phase 2.1 - Counting ===

    #[test]
    fn selected_count_returns_selected_leaves() {
        let mut parent = TreeNode::new("Home");
        parent.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        parent.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));

        parent.children[0].select();

        assert_eq!(parent.selected_count(), 1);
    }

    #[test]
    fn total_count_returns_all_leaves() {
        let mut parent = TreeNode::new("Home");
        parent.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        parent.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));

        assert_eq!(parent.total_count(), 2);
    }

    // === TDD: Phase 2.1 - Paths ===

    #[test]
    fn selected_paths_returns_only_selected() {
        let mut parent = TreeNode::new("Home");
        parent.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        parent.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));

        parent.children[0].select();

        let paths = parent.selected_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], PathBuf::from("a.md"));
    }

    // === TDD: Phase 2.1 - Bulk Operations ===

    #[test]
    fn select_all_selects_everything() {
        let mut root = TreeNode::new("Root");
        let mut home = TreeNode::new("Home");
        home.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        root.add_child(home);

        root.select_all();

        assert_eq!(root.selected_count(), 1);
        assert_eq!(root.total_count(), 1);
    }

    #[test]
    fn select_none_deselects_everything() {
        let mut root = TreeNode::new("Root");
        let mut home = TreeNode::new("Home");
        home.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        root.add_child(home);
        root.select_all();

        root.select_none();

        assert_eq!(root.selected_count(), 0);
    }

    #[test]
    fn invert_flips_selection() {
        let mut root = TreeNode::new("Root");
        root.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        root.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));

        root.children[0].select();
        assert_eq!(root.selected_count(), 1);

        root.invert();
        assert_eq!(root.selected_count(), 1);
        assert_eq!(root.children[0].state, SelectionState::Unselected);
        assert_eq!(root.children[1].state, SelectionState::Selected);
    }

    // === TDD: Phase 2.1 - Expansion ===

    #[test]
    fn expand_sets_expanded() {
        let mut node = TreeNode::new("Home");
        node.expand();
        assert!(node.expanded);
    }

    #[test]
    fn collapse_clears_expanded() {
        let mut node = TreeNode::new("Home");
        node.expand();
        node.collapse();
        assert!(!node.expanded);
    }

    #[test]
    fn toggle_expand_toggles() {
        let mut node = TreeNode::new("Home");
        node.toggle_expand();
        assert!(node.expanded);
        node.toggle_expand();
        assert!(!node.expanded);
    }
}
