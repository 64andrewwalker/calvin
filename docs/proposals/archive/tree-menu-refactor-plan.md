# Tree Menu Refactor Plan

> **Architecture Reference**: See `docs/architecture/layers.md` and `docs/architecture/directory.md`

## Current State

`src/ui/widgets/tree_menu.rs` is 1404 lines, exceeding the 400-line guideline.

Per `docs/architecture/file-size-guidelines.md`:
- **Current**: 1404 lines total, ~700 lines implementation, ~630 lines tests → 🔴 ALARM
- **Target**: Each module < 400 lines total

### Architecture Context

According to `docs/architecture/layers.md`:
- **ui/** belongs to **Layer 0: Presentation**
- UI components should focus on **rendering** and **terminal interaction**
- They should NOT contain business logic (that belongs in Layer 1/2)

### Analysis

| Component | Lines | Layer Responsibility |
|-----------|-------|---------------------|
| TreeNode struct + impl | ~180 | Data structure (could be domain-ish, but OK in presentation for UI-only use) |
| FlattenedNode struct | ~20 | Rendering view model |
| TreeAction enum | ~25 | UI action types |
| TreeMenu struct + impl | ~220 | Menu state and action handling |
| key_to_action | ~20 | Key mapping (presentation) |
| run_interactive | ~100 | Terminal interaction loop (presentation) |
| render_tree_node | ~60 | Single node rendering (presentation) |
| build_tree_from_lockfile | ~75 | Tree building from lockfile entries (borderline - uses lockfile keys) |
| infer_target_from_path | ~25 | Target inference (presentation helper) |
| Tests | ~630 | Test coverage |

**Total implementation: ~700 lines**
**Total tests: ~630 lines**

## Architecture Decision

The `tree_menu` component is **purely a Presentation layer widget**:
- It does NOT perform I/O (file reading/writing)
- It does NOT implement domain logic
- It receives data from `commands/clean.rs` which is the glue layer

This follows the existing pattern in `ui/widgets/` (box.rs, list.rs).

## Proposed Structure

```
src/ui/widgets/
├── tree_menu/
│   ├── mod.rs           # Public exports (re-exports all public items)
│   ├── node.rs          # TreeNode, SelectionState (~200 lines)
│   ├── menu.rs          # TreeMenu, TreeAction, FlattenedNode (~250 lines)
│   ├── render.rs        # render_tree_node, status bar, help bar (~100 lines)
│   ├── input.rs         # key_to_action, run_interactive (~130 lines)
│   └── builder.rs       # build_tree_from_lockfile, infer_target_from_path (~100 lines)
├── box.rs               # (existing)
├── list.rs              # (existing)
└── mod.rs               # Update: `pub mod tree_menu;`
```

This mirrors the existing pattern where complex widgets can be directories (like `commands/deploy/`).

### Module Responsibilities

#### node.rs (~200 lines)
**Purpose**: Core data structure for tree selection

- `SelectionState` enum - Selection state values
- `TreeNode` struct and all its methods:
  - `new`, `leaf`, `is_leaf` - Constructors
  - `add_child` - Tree building
  - `select`, `deselect`, `toggle` - Selection operations
  - `update_state_from_children` - State propagation
  - `selected_count`, `total_count` - Counting
  - `selected_paths`, `selected_keys` - Querying
  - `expand`, `collapse`, `toggle_expand` - Expansion
  - `select_all`, `select_none`, `invert` - Bulk operations

**Dependencies**: None (self-contained)

#### menu.rs (~250 lines)
**Purpose**: Menu state management and action handling

- `FlattenedNode` struct - Flattened view for rendering
- `TreeAction` enum - All possible UI actions
- `TreeMenu` struct and methods:
  - `new`, `rebuild_flattened`, `flatten_node` - Initialization
  - `flattened_nodes`, `cursor_position` - State accessors
  - `handle_action` - Action dispatcher
  - `get_node_mut`, `get_node_at_path` - Tree traversal
  - `propagate_state_changes`, `update_parent_states` - State sync
  - `selected_paths`, `selected_keys` - Query delegation
  - `selected_count`, `total_count` - Count delegation

**Dependencies**: `node.rs`

#### render.rs (~100 lines)
**Purpose**: Terminal rendering functions

- `render_tree_node` function - Single node to string
- Icon selection logic (Unicode vs ASCII)
- `render_status_bar` - Selection status display
- `render_help_bar` - Keyboard shortcut hints

**Dependencies**: `node.rs` (for `SelectionState`), `crate::ui::theme`

#### input.rs (~130 lines)
**Purpose**: Terminal input handling

- `key_to_action` function - KeyEvent to TreeAction mapping
- `run_interactive` function - Main terminal interaction loop
  - Raw mode setup/teardown
  - Event loop
  - Screen clearing and redrawing

**Dependencies**: `menu.rs`, `render.rs`, `crossterm`

#### builder.rs (~100 lines)
**Purpose**: Tree construction from external data

- `build_tree_from_lockfile` function - Build tree from (key, path) pairs
- `infer_target_from_path` helper - Path pattern matching

**Dependencies**: `node.rs`

### Test Strategy

Tests will remain in the module files with `#[cfg(test)] mod tests`.

| Module | Test Focus | Key Tests |
|--------|------------|-----------|
| node.rs | TreeNode behavior | Selection cascading, state propagation, counting |
| menu.rs | Menu actions | Navigation, action handling, flattening |
| render.rs | Output format | Icon rendering, ASCII fallback |
| input.rs | Key mapping | Key-to-action mapping (unit only, no interactive) |
| builder.rs | Tree construction | Scope grouping, target inference |

## Refactoring TODO

### Phase 1: Preparation (test coverage)
- [ ] Add missing edge case tests for TreeNode
  - [ ] Deep nesting (3+ levels)
  - [ ] Empty tree
  - [ ] Single node tree
- [ ] Add missing edge case tests for TreeMenu
  - [ ] Empty menu (no children)
  - [ ] Cursor at boundaries after expand/collapse
  - [ ] Action on collapsed node
- [ ] Add tests for render functions
  - [ ] Partial selection icon
  - [ ] Deep indentation rendering
  - [ ] Long labels
- [ ] Add tests for builder functions
  - [ ] Empty entries
  - [ ] Only home entries
  - [ ] Only project entries
  - [ ] Unknown target paths

### Phase 2: Extract modules
- [ ] Create `src/ui/widgets/tree_menu/` directory
- [ ] Create `mod.rs` with public exports
- [ ] Move `SelectionState` and `TreeNode` to `node.rs`
- [ ] Move `FlattenedNode`, `TreeAction`, `TreeMenu` to `menu.rs`
- [ ] Move `render_tree_node` and related to `render.rs`
- [ ] Move `key_to_action`, `run_interactive` to `input.rs`
- [ ] Move `build_tree_from_lockfile`, `infer_target_from_path` to `builder.rs`

### Phase 3: Update imports
- [ ] Update `src/ui/widgets/mod.rs`
- [ ] Update `src/commands/clean.rs` imports
- [ ] Run `cargo test` to verify

### Phase 4: Cleanup
- [ ] Remove `#![allow(dead_code)]` if no longer needed
- [ ] Verify each module is under 400 lines
- [ ] Run full pre-commit checks

## Success Criteria

1. All existing tests pass
2. No new clippy warnings
3. Each module < 400 lines
4. Public API unchanged (backward compatible)
5. Code coverage maintained

## Risk Assessment

| Risk | Mitigation |
|------|------------|
| Breaking changes | Run tests after each extraction |
| Import cycles | Clear module boundaries, use pub(crate) |
| Test failures | Extract tests with their code |

## Estimated Effort

- Phase 1: 30 min (add tests)
- Phase 2: 45 min (extract modules)
- Phase 3: 15 min (update imports)
- Phase 4: 10 min (cleanup)

**Total: ~100 min**

