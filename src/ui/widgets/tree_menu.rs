//! Tree Menu Widget
//!
//! A hierarchical tree structure for selecting items in the clean command.
//! Supports parent-child selection propagation and partial selection states.

#![allow(dead_code)] // Phase 2: Tree menu not yet integrated into clean command

use std::path::{Path, PathBuf};

use crate::ui::theme::{icons, icons_ascii};

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

    /// Render the help bar
    pub fn render_help_bar(&self) -> String {
        String::from(
            "[a] All    [n] None    [i] Invert    [Enter] Confirm    [q] Quit\n\
             (Use ↑↓ to navigate, Space to toggle, →← to expand/collapse)",
        )
    }
}

/// Convert a keyboard event to a TreeAction
pub fn key_to_action(key: crossterm::event::KeyEvent) -> Option<TreeAction> {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(TreeAction::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(TreeAction::Down),
        KeyCode::Char(' ') => Some(TreeAction::Toggle),
        KeyCode::Right | KeyCode::Char('l') => Some(TreeAction::Expand),
        KeyCode::Left | KeyCode::Char('h') | KeyCode::Backspace => Some(TreeAction::Collapse),
        KeyCode::Char('a') => Some(TreeAction::SelectAll),
        KeyCode::Char('n') => Some(TreeAction::SelectNone),
        KeyCode::Char('i') => Some(TreeAction::Invert),
        KeyCode::Char('q') | KeyCode::Esc => Some(TreeAction::Quit),
        _ => None,
    }
}

/// Run the tree menu interactively
/// Returns the selected keys if confirmed, None if quit
pub fn run_interactive(
    menu: &mut TreeMenu,
    supports_unicode: bool,
) -> std::io::Result<Option<Vec<String>>> {
    use crossterm::{
        cursor,
        event::{self, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{self, ClearType},
    };
    use std::io::{stdout, Write};

    // Enable raw mode
    terminal::enable_raw_mode()?;
    let mut stdout = stdout();

    // Helper to render the full UI
    let render_ui = |stdout: &mut std::io::Stdout, menu: &TreeMenu| -> std::io::Result<()> {
        // Clear entire screen and move to top
        execute!(
            stdout,
            terminal::Clear(ClearType::All),
            cursor::MoveTo(0, 0)
        )?;

        // Header
        println!("🧹 Calvin Clean\r");
        println!("\r");

        // Render tree
        let rendered = menu.render(supports_unicode);
        for line in rendered.lines() {
            print!("{}\r\n", line);
        }

        // Separator
        println!("───────────────────────────────────────────────────────────────\r");

        // Status bar
        let status = menu.render_status_bar(supports_unicode);
        for line in status.lines() {
            print!("{}\r\n", line);
        }
        println!("\r");

        // Help bar
        let help = menu.render_help_bar();
        for line in help.lines() {
            print!("{}\r\n", line);
        }

        stdout.flush()?;
        Ok(())
    };

    // Hide cursor
    execute!(stdout, cursor::Hide)?;

    // Initial render
    render_ui(&mut stdout, menu)?;

    let result = loop {
        // Wait for key event
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            // Enter always confirms selection
            if key.code == KeyCode::Enter {
                break Some(menu.selected_keys());
            }

            if let Some(action) = key_to_action(key) {
                match action {
                    TreeAction::Confirm => break Some(menu.selected_keys()),
                    TreeAction::Quit => break None,
                    _ => {
                        menu.handle_action(action);
                        // Redraw after action
                        render_ui(&mut stdout, menu)?;
                    }
                }
            }
        }
    };

    // Restore terminal
    execute!(
        stdout,
        cursor::Show,
        terminal::Clear(ClearType::All),
        cursor::MoveTo(0, 0)
    )?;
    terminal::disable_raw_mode()?;

    Ok(result)
}

/// Render a single tree node
fn render_tree_node(node: &FlattenedNode, is_active: bool, supports_unicode: bool) -> String {
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

/// Build a tree from lockfile entries
///
/// Groups entries by Scope → Target (inferred from path).
/// Since the lockfile doesn't store target info, we infer from the path prefix.
pub fn build_tree_from_lockfile(entries: impl IntoIterator<Item = (String, PathBuf)>) -> TreeNode {
    use std::collections::HashMap;

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
fn infer_target_from_path(path: &Path) -> String {
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

    // === TDD: Phase 2.2 - TreeMenu ===

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
    fn render_tree_structure_with_real_entries() {
        // Create entries with various targets for each scope
        let entries = vec![
            // Claude-code home entries
            (
                "home:~/.claude/commands/test1.md".to_string(),
                PathBuf::from("/Users/test/.claude/commands/test1.md"),
            ),
            (
                "home:~/.claude/commands/test2.md".to_string(),
                PathBuf::from("/Users/test/.claude/commands/test2.md"),
            ),
            // Cursor home entries
            (
                "home:~/.cursor/rules/prompt1.mdc".to_string(),
                PathBuf::from("/Users/test/.cursor/rules/prompt1.mdc"),
            ),
            // VSCode home entries
            (
                "home:~/.vscode/instructions/task.instructions.md".to_string(),
                PathBuf::from("/Users/test/.vscode/instructions/task.instructions.md"),
            ),
            // Codex home entries
            (
                "home:~/.codex/prompts/help.md".to_string(),
                PathBuf::from("/Users/test/.codex/prompts/help.md"),
            ),
            // Antigravity home entries
            (
                "home:~/.gemini/antigravity/global_workflows/flow.md".to_string(),
                PathBuf::from("/Users/test/.gemini/antigravity/global_workflows/flow.md"),
            ),
            // Project entries
            (
                "project:.cursor/rules/local.mdc".to_string(),
                PathBuf::from(".cursor/rules/local.mdc"),
            ),
        ];

        let root = build_tree_from_lockfile(entries);
        let mut menu = TreeMenu::new(root);

        // Expand Home node to show targets
        menu.handle_action(TreeAction::Down); // Move to Home
        menu.handle_action(TreeAction::Expand); // Expand Home

        // Print the rendered tree
        println!("\n=== Tree Menu Structure ===\n");
        let rendered = menu.render(true);
        println!("{}", rendered);
        println!("───────────────────────────────────────────────────────────────");
        println!("{}", menu.render_status_bar(true));
        println!("\n{}", menu.render_help_bar());

        // Verify tree structure
        assert_eq!(menu.total_count(), 7);
    }

    // === TDD: Phase 2.3 - Rendering ===

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
        let root = create_test_tree();
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
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);

        // Expand Home
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Expand);

        let rendered = menu.render(true);
        assert!(rendered.contains("▼"), "Should show expanded icon");
    }

    #[test]
    fn render_status_bar_shows_counts() {
        let root = create_test_tree();
        let mut menu = TreeMenu::new(root);
        menu.handle_action(TreeAction::SelectAll);

        let status = menu.render_status_bar(true);
        assert!(status.contains("3/3"), "Should show 3/3 selected");
    }

    #[test]
    fn render_help_bar_shows_shortcuts() {
        let root = create_test_tree();
        let menu = TreeMenu::new(root);

        let help = menu.render_help_bar();
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

    // === Phase 1: Edge Case Tests for Refactoring ===

    // --- TreeNode Edge Cases ---

    #[test]
    fn node_deep_nesting_selection_propagates() {
        // Build 4-level tree: root -> level1 -> level2 -> leaf
        let mut root = TreeNode::new("Root");
        let mut level1 = TreeNode::new("Level1");
        let mut level2 = TreeNode::new("Level2");
        level2.add_child(TreeNode::leaf(
            "deep.md",
            PathBuf::from("deep.md"),
            "home:deep.md".to_string(),
        ));
        level1.add_child(level2);
        root.add_child(level1);

        // Select root should cascade down all levels
        root.select();

        assert_eq!(root.state, SelectionState::Selected);
        assert_eq!(root.children[0].state, SelectionState::Selected); // level1
        assert_eq!(root.children[0].children[0].state, SelectionState::Selected); // level2
        assert_eq!(
            root.children[0].children[0].children[0].state,
            SelectionState::Selected
        ); // leaf
    }

    #[test]
    fn node_deep_nesting_partial_propagates_up() {
        // Build 4-level tree with 2 leaves
        let mut root = TreeNode::new("Root");
        let mut level1 = TreeNode::new("Level1");
        let mut level2 = TreeNode::new("Level2");
        level2.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "home:a.md".to_string(),
        ));
        level2.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "home:b.md".to_string(),
        ));
        level1.add_child(level2);
        root.add_child(level1);

        // Select only one leaf
        root.children[0].children[0].children[0].select();

        // Propagate upward
        root.children[0].children[0].update_state_from_children();
        root.children[0].update_state_from_children();
        root.update_state_from_children();

        assert_eq!(root.state, SelectionState::Partial);
        assert_eq!(root.children[0].state, SelectionState::Partial);
        assert_eq!(root.children[0].children[0].state, SelectionState::Partial);
    }

    #[test]
    fn node_update_state_from_children_no_children() {
        let mut leaf = TreeNode::leaf(
            "test.md",
            PathBuf::from("test.md"),
            "home:test.md".to_string(),
        );

        // Should return false and not change state
        let changed = leaf.update_state_from_children();
        assert!(!changed);
        assert_eq!(leaf.state, SelectionState::Unselected);
    }

    #[test]
    fn node_selected_paths_empty_tree() {
        let root = TreeNode::new("Empty");
        let paths = root.selected_paths();
        assert!(paths.is_empty());
    }

    #[test]
    fn node_selected_keys_deeply_nested() {
        let mut root = TreeNode::new("Root");
        let mut level1 = TreeNode::new("Level1");
        let mut level2 = TreeNode::new("Level2");
        level2.add_child(TreeNode::leaf(
            "deep.md",
            PathBuf::from("deep.md"),
            "home:deep.md".to_string(),
        ));
        level1.add_child(level2);
        root.add_child(level1);

        root.select();
        let keys = root.selected_keys();

        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "home:deep.md");
    }

    #[test]
    fn node_total_count_deeply_nested() {
        let mut root = TreeNode::new("Root");
        let mut level1 = TreeNode::new("Level1");
        let mut level2a = TreeNode::new("Level2a");
        let mut level2b = TreeNode::new("Level2b");

        level2a.add_child(TreeNode::leaf(
            "a.md",
            PathBuf::from("a.md"),
            "a".to_string(),
        ));
        level2a.add_child(TreeNode::leaf(
            "b.md",
            PathBuf::from("b.md"),
            "b".to_string(),
        ));
        level2b.add_child(TreeNode::leaf(
            "c.md",
            PathBuf::from("c.md"),
            "c".to_string(),
        ));

        level1.add_child(level2a);
        level1.add_child(level2b);
        root.add_child(level1);

        assert_eq!(root.total_count(), 3);
    }

    #[test]
    fn node_invert_on_leaf() {
        let mut leaf = TreeNode::leaf("a.md", PathBuf::from("a.md"), "home:a.md".to_string());

        // Initially unselected
        assert_eq!(leaf.state, SelectionState::Unselected);

        // Invert should select
        leaf.invert();
        assert_eq!(leaf.state, SelectionState::Selected);

        // Invert again should deselect
        leaf.invert();
        assert_eq!(leaf.state, SelectionState::Unselected);
    }

    // --- TreeMenu Edge Cases ---

    #[test]
    fn menu_empty_tree() {
        let root = TreeNode::new("Empty");
        let menu = TreeMenu::new(root);

        assert_eq!(menu.flattened_nodes().len(), 1); // Just root
                                                     // A non-leaf node with no children still counts as 1 in total_count
                                                     // (because is_leaf returns true when children is empty)
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

    // --- Render Edge Cases ---

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
        // Format: "  " cursor + depth * "  " indent
        let deep_line = rendered.lines().last().unwrap();
        assert!(
            deep_line.contains("          "), // 10 spaces of indentation
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
        let root = create_test_tree();
        let menu = TreeMenu::new(root);

        let status = menu.render_status_bar(true);
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
        menu.handle_action(TreeAction::Down);
        menu.handle_action(TreeAction::Expand);

        let rendered_expanded = menu.render(true);
        assert!(
            rendered_expanded.contains("▼"),
            "Expanded should show ▼: {}",
            rendered_expanded
        );
    }

    // --- Input Edge Cases ---

    #[test]
    fn key_to_action_arrow_keys() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
            Some(TreeAction::Up)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)),
            Some(TreeAction::Down)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
            Some(TreeAction::Collapse)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)),
            Some(TreeAction::Expand)
        );
    }

    #[test]
    fn key_to_action_vim_keys() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE)),
            Some(TreeAction::Up)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE)),
            Some(TreeAction::Down)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE)),
            Some(TreeAction::Collapse)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE)),
            Some(TreeAction::Expand)
        );
    }

    #[test]
    fn key_to_action_bulk_shortcuts() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(TreeAction::SelectAll)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)),
            Some(TreeAction::SelectNone)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
            Some(TreeAction::Invert)
        );
    }

    #[test]
    fn key_to_action_quit_keys() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
            Some(TreeAction::Quit)
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Some(TreeAction::Quit)
        );
    }

    #[test]
    fn key_to_action_unknown_key() {
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            key_to_action(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)),
            None
        );
    }

    // --- Builder Edge Cases ---

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
        // Test all target inference patterns
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
}
