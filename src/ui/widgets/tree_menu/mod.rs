//! Tree Menu Widget
//!
//! A hierarchical tree structure for selecting items in the clean command.
//! Supports parent-child selection propagation and partial selection states.
//!
//! # Module Structure
//!
//! - `node` - TreeNode data structure and selection logic
//! - `menu` - TreeMenu state management and action handling
//! - `render` - Terminal rendering functions
//! - `input` - Keyboard input handling and interactive loop
//! - `builder` - Tree construction from lockfile entries

// Allow dead code and unused imports - public API is used by clean command's interactive mode
// Some exports are for future use or internal module communication
#![allow(dead_code, unused_imports)]

mod builder;
mod input;
mod menu;
mod node;
mod render;

// Re-export public API
pub use builder::build_tree_from_lockfile;
pub use input::{key_to_action, run_interactive};
pub use menu::{FlattenedNode, TreeAction, TreeMenu};
pub use node::{SelectionState, TreeNode};

// Re-export render functions for TreeMenu to use
pub(crate) use render::{render_help_bar, render_status_bar, render_tree_node};
