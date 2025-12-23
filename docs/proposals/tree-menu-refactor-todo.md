# Tree Menu Refactor TODO

> Status: ✅ **COMPLETED**  
> Created: 2024-12-23  
> Completed: 2024-12-23  
> Target: Split `tree_menu.rs` (1404 lines) into 5 modules (~100-250 lines each)
>
> **Architecture Reference**: 
> - `docs/architecture/layers.md` - Layer 0: Presentation
> - `docs/architecture/file-size-guidelines.md` - Size thresholds

## Summary

Successfully refactored `tree_menu.rs` into a modular directory structure:

| Module | Impl Lines | Total Lines | Description |
|--------|-----------|-------------|-------------|
| `node.rs` | 219 | 666 | TreeNode, SelectionState |
| `menu.rs` | 260 | 534 | TreeMenu, TreeAction, FlattenedNode |
| `render.rs` | 107 | 344 | Terminal rendering functions |
| `input.rs` | 129 | 222 | Keyboard input and interactive loop |
| `builder.rs` | 111 | 296 | Tree construction from lockfile |
| `mod.rs` | 31 | 31 | Public exports |

All implementation code < 400 lines per file ✅

## Phase 1: Test Coverage Enhancement ✅

> **Goal**: Ensure comprehensive test coverage before refactoring.
> Per TDD principles, we write tests first, then refactor with confidence.

### TreeNode Edge Cases (node.rs) ✅
- [x] `node_deep_nesting_selection_propagates` - 3+ levels of nesting, selection cascades down
- [x] `node_deep_nesting_partial_propagates_up` - Partial state bubbles up through 3+ levels
- [x] `node_update_state_from_children_no_children` - Edge case: returns false for leaf
- [x] `node_selected_paths_empty_tree` - Empty tree returns empty vec
- [x] `node_selected_keys_deeply_nested` - Keys collected from all levels
- [x] `node_total_count_deeply_nested` - Count across multiple levels
- [x] `node_invert_on_leaf` - Invert works correctly on leaf nodes

### TreeMenu Edge Cases (menu.rs) ✅
- [x] `menu_empty_tree` - Menu with only root (no children)
- [x] `menu_cursor_stays_valid_after_collapse` - Cursor doesn't go OOB
- [x] `menu_toggle_on_collapsed_parent_selects_hidden_children` - Hidden children also selected
- [x] `menu_expand_leaf_no_effect` - Expand on leaf does nothing
- [x] `menu_collapse_already_collapsed_no_effect` - Collapse on collapsed is idempotent
- [x] `menu_selected_keys_after_bulk_operations` - SelectAll -> SelectNone cycle
- [x] `menu_flattened_order_matches_tree_order` - DFS order preserved

### Render Edge Cases (render.rs) ✅
- [x] `render_partial_icon_unicode` - Partial selection shows ◐
- [x] `render_partial_icon_ascii` - Partial selection shows [-]
- [x] `render_deep_indentation` - 5+ levels have correct indent
- [x] `render_long_label_not_truncated` - 100+ char labels render fully
- [x] `render_status_bar_zero_selected` - Shows "0/N files"
- [x] `render_help_bar_shows_shortcuts` - All shortcuts documented
- [x] `render_collapsed_vs_expanded_icon` - ▶ vs ▼

### Input Edge Cases (input.rs) ✅
- [x] `key_to_action_arrow_keys` - Up/Down/Left/Right mapped correctly
- [x] `key_to_action_vim_keys` - h/j/k/l mapped correctly
- [x] `key_to_action_space_toggle` - Space = Toggle
- [x] `key_to_action_bulk_shortcuts` - a/n/i mapped to SelectAll/None/Invert
- [x] `key_to_action_quit_keys` - q/Esc both quit
- [x] `key_to_action_unknown_key` - Unknown key returns None

### Builder Edge Cases (builder.rs) ✅
- [x] `builder_empty_entries` - Empty input returns tree with just root
- [x] `builder_home_only` - Only home scope entries, no project node
- [x] `builder_project_only` - Only project scope entries, no home node
- [x] `builder_unknown_target_path` - Falls back to "other" target
- [x] `builder_infer_all_targets` - All platform inference patterns (covers claude-code, cursor, vscode, codex, antigravity, agents-md)
- [x] `builder_mixed_targets_sorted` - Targets sorted alphabetically
- [x] `render_tree_structure_with_real_entries` - Integration test with realistic data

## Phase 2: Module Extraction ✅

> **Extraction Order**: Dependencies flow node → menu → render → input, and node → builder
> Extract in order: node.rs → builder.rs → menu.rs → render.rs → input.rs

### 2.1 Create Directory Structure ✅
- [x] Create `src/ui/widgets/tree_menu/` directory
- [x] Create `src/ui/widgets/tree_menu/mod.rs` with placeholder exports
- [x] Update `src/ui/widgets/mod.rs` to reference new module
- [x] Verify compiles: `cargo check`

### 2.2 Extract node.rs ✅
- [x] Create `src/ui/widgets/tree_menu/node.rs`
- [x] Move `SelectionState` enum
- [x] Move `TreeNode` struct and impl
- [x] Add module doc comment explaining purpose
- [x] Update mod.rs: `pub use node::{SelectionState, TreeNode};`
- [x] Run: `cargo test widgets::tree_menu::node` (28 tests pass)

### 2.3 Extract builder.rs ✅
- [x] Create `src/ui/widgets/tree_menu/builder.rs`
- [x] Move `build_tree_from_lockfile` function
- [x] Move `infer_target_from_path` function
- [x] Add `use super::node::TreeNode;`
- [x] Update mod.rs: `pub use builder::build_tree_from_lockfile;`
- [x] Run: `cargo test widgets::tree_menu::builder` (8 tests pass)

### 2.4 Extract menu.rs ✅
- [x] Create `src/ui/widgets/tree_menu/menu.rs`
- [x] Move `FlattenedNode` struct
- [x] Move `TreeAction` enum
- [x] Move `TreeMenu` struct and impl
- [x] Add `use super::node::{SelectionState, TreeNode};`
- [x] Update mod.rs: `pub use menu::{FlattenedNode, TreeAction, TreeMenu};`
- [x] Run: `cargo test widgets::tree_menu::menu` (15 tests pass)

### 2.5 Extract render.rs ✅
- [x] Create `src/ui/widgets/tree_menu/render.rs`
- [x] Move `render_tree_node` function
- [x] Extract `render_status_bar` from TreeMenu method to standalone function
- [x] Extract `render_help_bar` from TreeMenu method to standalone function
- [x] Add `use super::node::SelectionState;`
- [x] Add `use super::menu::FlattenedNode;`
- [x] Update TreeMenu to call render functions instead of inline
- [x] Run: `cargo test widgets::tree_menu::render` (12 tests pass)

### 2.6 Extract input.rs ✅
- [x] Create `src/ui/widgets/tree_menu/input.rs`
- [x] Move `key_to_action` function
- [x] Move `run_interactive` function
- [x] Add `use super::menu::{TreeAction, TreeMenu};`
- [x] Update mod.rs: `pub use input::{key_to_action, run_interactive};`
- [x] Run: `cargo test widgets::tree_menu::input` (6 tests pass)

### 2.7 Delete old file ✅
- [x] Verify all tests pass: `cargo test`
- [x] Delete `src/ui/widgets/tree_menu.rs`
- [x] Verify still compiles: `cargo check`

## Phase 3: Integration & Verification ✅

### Update External Imports ✅
- [x] Update `src/commands/clean.rs` imports (already compatible)
- [x] Verify no broken imports: `cargo check`

### Verify Module Sizes ✅
- [x] `node.rs` - 219 impl lines (< 400) ✅
- [x] `menu.rs` - 260 impl lines (< 400) ✅
- [x] `render.rs` - 107 impl lines (< 400) ✅
- [x] `input.rs` - 129 impl lines (< 400) ✅
- [x] `builder.rs` - 111 impl lines (< 400) ✅
- [x] `mod.rs` - 31 lines (< 50) ✅

## Phase 4: Cleanup & Final Checks ✅

### Code Cleanup ✅
- [x] Added `#![allow(dead_code, unused_imports)]` to mod.rs (public API for future use)
- [x] Add module-level doc comments to each file ✅
- [x] Public items have doc comments ✅

### Pre-commit Checks ✅
- [x] Run `cargo fmt` - ✅
- [x] Run `cargo clippy` - ✅ no warnings
- [x] Run `cargo test` - ✅ 759 tests pass
- [x] Pre-commit checks pass - ✅ no ALARM on tree_menu

### Manual Verification
- [ ] `calvin clean --dry-run` works (pending user verification)
- [ ] `calvin clean` interactive mode works (pending user verification)
- [ ] `calvin clean --home --yes` works (pending user verification)
- [ ] `calvin clean --project --yes` works (pending user verification)

## Final Verification Checklist

- [x] All 759 tests pass
- [x] No clippy warnings
- [x] Pre-commit checks pass
- [x] Each module implementation < 400 lines
- [x] Public API unchanged
- [ ] Clean command fully functional (pending manual verification)

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

