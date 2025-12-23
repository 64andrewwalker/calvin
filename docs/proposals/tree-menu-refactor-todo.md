# Tree Menu Refactor TODO

> Status: In Progress  
> Created: 2024-12-23  
> Target: Split `tree_menu.rs` (1404 lines) into 5 modules (~100-250 lines each)
>
> **Architecture Reference**: 
> - `docs/architecture/layers.md` - Layer 0: Presentation
> - `docs/architecture/file-size-guidelines.md` - Size thresholds

## Phase 1: Test Coverage Enhancement

> **Goal**: Ensure comprehensive test coverage before refactoring.
> Per TDD principles, we write tests first, then refactor with confidence.

### TreeNode Edge Cases (node.rs)
- [ ] `node_deep_nesting_selection_propagates` - 3+ levels of nesting, selection cascades down
- [ ] `node_deep_nesting_partial_propagates_up` - Partial state bubbles up through 3+ levels
- [ ] `node_empty_children_is_leaf` - Verify empty children = leaf
- [ ] `node_update_state_from_children_no_children` - Edge case: returns false for leaf
- [ ] `node_selected_paths_empty_tree` - Empty tree returns empty vec
- [ ] `node_selected_keys_deeply_nested` - Keys collected from all levels
- [ ] `node_total_count_deeply_nested` - Count across multiple levels
- [ ] `node_invert_on_leaf` - Invert works correctly on leaf nodes

### TreeMenu Edge Cases (menu.rs)
- [ ] `menu_empty_tree` - Menu with only root (no children)
- [ ] `menu_cursor_stays_valid_after_collapse` - Cursor doesn't go OOB
- [ ] `menu_cursor_stays_valid_after_expand` - Cursor position maintained
- [ ] `menu_toggle_on_collapsed_parent_selects_hidden_children` - Hidden children also selected
- [ ] `menu_expand_leaf_no_effect` - Expand on leaf does nothing
- [ ] `menu_collapse_already_collapsed_no_effect` - Collapse on collapsed is idempotent
- [ ] `menu_selected_keys_empty` - No selection returns empty vec
- [ ] `menu_selected_keys_after_bulk_operations` - SelectAll -> SelectNone cycle
- [ ] `menu_flattened_order_matches_tree_order` - DFS order preserved

### Render Edge Cases (render.rs)
- [ ] `render_partial_icon_unicode` - Partial selection shows ◐
- [ ] `render_partial_icon_ascii` - Partial selection shows [~]
- [ ] `render_deep_indentation` - 5+ levels have correct indent
- [ ] `render_long_label_not_truncated` - 100+ char labels render fully
- [ ] `render_status_bar_zero_selected` - Shows "0/N files"
- [ ] `render_status_bar_all_selected` - Shows "N/N files"
- [ ] `render_help_bar_has_all_shortcuts` - All shortcuts documented
- [ ] `render_collapsed_vs_expanded_icon` - ▶ vs ▼

### Input Edge Cases (input.rs)
- [ ] `key_to_action_arrow_keys` - Up/Down/Left/Right mapped correctly
- [ ] `key_to_action_vim_keys` - h/j/k/l mapped correctly
- [ ] `key_to_action_space_toggle` - Space = Toggle
- [ ] `key_to_action_bulk_shortcuts` - a/n/i mapped to SelectAll/None/Invert
- [ ] `key_to_action_quit_keys` - q/Esc both quit
- [ ] `key_to_action_unknown_key` - Unknown key returns None

### Builder Edge Cases (builder.rs)
- [ ] `builder_empty_entries` - Empty input returns tree with just root
- [ ] `builder_home_only` - Only home scope entries, no project node
- [ ] `builder_project_only` - Only project scope entries, no home node
- [ ] `builder_unknown_target_path` - Falls back to "other" target
- [ ] `builder_windows_paths` - Handles backslash paths correctly
- [ ] `builder_mixed_targets_sorted` - Targets sorted alphabetically
- [ ] `builder_infer_claude_code` - `.claude/` detected as claude-code
- [ ] `builder_infer_cursor` - `.cursor/` detected as cursor
- [ ] `builder_infer_vscode` - `.vscode/` detected as vscode
- [ ] `builder_infer_codex` - `.codex/` detected as codex
- [ ] `builder_infer_antigravity` - `.gemini/` detected as antigravity
- [ ] `builder_infer_agents_md` - `AGENTS.md` detected as agents-md

## Phase 2: Module Extraction

> **Extraction Order**: Dependencies flow node → menu → render → input, and node → builder
> Extract in order: node.rs → builder.rs → menu.rs → render.rs → input.rs

### 2.1 Create Directory Structure
- [ ] Create `src/ui/widgets/tree_menu/` directory
- [ ] Create `src/ui/widgets/tree_menu/mod.rs` with placeholder exports
- [ ] Update `src/ui/widgets/mod.rs` to reference new module
- [ ] Verify compiles: `cargo check`

### 2.2 Extract node.rs (first - no dependencies)
- [ ] Create `src/ui/widgets/tree_menu/node.rs`
- [ ] Move `SelectionState` enum
- [ ] Move `TreeNode` struct and impl
- [ ] Add module doc comment explaining purpose
- [ ] Update mod.rs: `pub use node::{SelectionState, TreeNode};`
- [ ] Run: `cargo test widgets::tree_menu::node`

### 2.3 Extract builder.rs (depends on node.rs only)
- [ ] Create `src/ui/widgets/tree_menu/builder.rs`
- [ ] Move `build_tree_from_lockfile` function
- [ ] Move `infer_target_from_path` function
- [ ] Add `use super::node::TreeNode;`
- [ ] Update mod.rs: `pub use builder::build_tree_from_lockfile;`
- [ ] Run: `cargo test widgets::tree_menu::builder`

### 2.4 Extract menu.rs (depends on node.rs)
- [ ] Create `src/ui/widgets/tree_menu/menu.rs`
- [ ] Move `FlattenedNode` struct
- [ ] Move `TreeAction` enum
- [ ] Move `TreeMenu` struct and impl
- [ ] Add `use super::node::{SelectionState, TreeNode};`
- [ ] Update mod.rs: `pub use menu::{FlattenedNode, TreeAction, TreeMenu};`
- [ ] Run: `cargo test widgets::tree_menu::menu`

### 2.5 Extract render.rs (depends on node.rs, uses theme)
- [ ] Create `src/ui/widgets/tree_menu/render.rs`
- [ ] Move `render_tree_node` function
- [ ] Extract `render_status_bar` from TreeMenu method to standalone function
- [ ] Extract `render_help_bar` from TreeMenu method to standalone function
- [ ] Add `use super::node::SelectionState;`
- [ ] Add `use super::menu::FlattenedNode;`
- [ ] Update TreeMenu to call render functions instead of inline
- [ ] Run: `cargo test widgets::tree_menu::render`

### 2.6 Extract input.rs (depends on menu.rs, render.rs)
- [ ] Create `src/ui/widgets/tree_menu/input.rs`
- [ ] Move `key_to_action` function
- [ ] Move `run_interactive` function
- [ ] Add `use super::menu::{TreeAction, TreeMenu};`
- [ ] Update mod.rs: `pub use input::{key_to_action, run_interactive};`
- [ ] Run: `cargo test widgets::tree_menu::input`

### 2.7 Delete old file
- [ ] Verify all tests pass: `cargo test`
- [ ] Delete `src/ui/widgets/tree_menu.rs`
- [ ] Verify still compiles: `cargo check`

## Phase 3: Integration & Verification

### Update External Imports
- [ ] Update `src/commands/clean.rs` imports
  - Change: `use crate::ui::widgets::tree_menu::{...}`
  - Verify: Same public API as before
- [ ] Verify no broken imports: `cargo check`

### Verify Module Sizes
- [ ] `node.rs` < 400 lines (target: ~200)
- [ ] `menu.rs` < 400 lines (target: ~250)
- [ ] `render.rs` < 400 lines (target: ~100)
- [ ] `input.rs` < 400 lines (target: ~130)
- [ ] `builder.rs` < 400 lines (target: ~100)
- [ ] `mod.rs` < 50 lines (just re-exports)

## Phase 4: Cleanup & Final Checks

### Code Cleanup
- [ ] Remove `#![allow(dead_code)]` if now used
- [ ] Add module-level doc comments to each file
- [ ] Ensure public items have `///` doc comments

### Pre-commit Checks
- [ ] Run `cargo fmt`
- [ ] Run `cargo clippy` - no warnings
- [ ] Run `cargo test` - all pass
- [ ] Run `./scripts/check-file-size.sh` - no ALARM on tree_menu

### Manual Verification
- [ ] `calvin clean --dry-run` works
- [ ] `calvin clean` interactive mode works (manual test)
- [ ] `calvin clean --home --yes` works
- [ ] `calvin clean --project --yes` works

## Final Verification Checklist

- [ ] All 533+ tests pass
- [ ] No clippy warnings
- [ ] Pre-commit checks pass
- [ ] Each module < 400 lines
- [ ] Public API unchanged
- [ ] Clean command fully functional

## Notes

### Dependencies Between Modules

```
builder.rs ──┐
             │
node.rs ─────┤
             ├──► menu.rs ──► render.rs
             │           │
input.rs ────┴───────────┘
```

- `node.rs`: No dependencies on other tree_menu modules
- `menu.rs`: Depends on `node.rs`, `render.rs`
- `render.rs`: Depends on `node.rs` types (SelectionState)
- `input.rs`: Depends on `menu.rs` (TreeMenu, TreeAction)
- `builder.rs`: Depends on `node.rs` (TreeNode)

### Public API (must remain stable)

```rust
// From mod.rs - these must remain accessible:
pub use node::{SelectionState, TreeNode};
pub use menu::{FlattenedNode, TreeAction, TreeMenu};
pub use input::{key_to_action, run_interactive};
pub use builder::build_tree_from_lockfile;
```

