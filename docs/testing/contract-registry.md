# Calvin Contract Registry

> The single source of truth for all promises Calvin makes to users.

---

## How to Use This Document

1. **Before implementing a feature**: Check if there's a contract that applies
2. **After fixing a bug**: Add a contract that prevents recurrence
3. **When writing tests**: Reference contracts by ID

---

## Contract Format

```
## [DOMAIN]-[NUMBER]: [Short Name]

**Promise**: What we guarantee to users.

**Violation Example**: What would happen if this contract is broken.

**Test Location**: `tests/contracts/[file].rs::[test_name]`

**Related Issues**: Links to bugs this contract prevents.
```

---

## Path Contracts

### PATH-001: Lockfile Location

**Promise**: The lockfile (`.calvin.lock` or `calvin.lock`) is always written to the promptpack source directory, regardless of the current working directory when `calvin` is invoked.

**Violation Example**: User runs `calvin deploy` from `src/lib/`, lockfile appears in `src/lib/` instead of project root.

**Test Location**: `tests/contracts/paths.rs::contract_lockfile_at_source_root`

**Related Issues**: #42, commit 9c02632

---

### PATH-002: Deployed Files Location

**Promise**: Files are deployed relative to the target directory (project root or home), never relative to the current working directory.

**Violation Example**: User runs `calvin deploy --home` from `~/projects/foo`, files appear in `~/projects/foo/.cursor/` instead of `~/.cursor/`.

**Test Location**: `tests/contracts/paths.rs::contract_deployed_files_at_target`

**Related Issues**: commit cdedded

---

### PATH-003: Display Path Tilde Notation

**Promise**: All user-facing output (stdout/stderr, except `--json`) uses `~` notation for paths under the user's home directory.

**Violation Example**: Output shows `/Users/alice/.calvin/.promptpack` instead of `~/.calvin/.promptpack`.

**Test Location**: `tests/contracts/paths.rs::contract_display_paths_use_tilde`

**Related Issues**: commit 96178a6

**Exception**: `--json` output uses absolute paths for machine parsing.

---

### PATH-004: JSON Path Absoluteness

**Promise**: All paths in `--json` output are absolute paths, suitable for programmatic consumption.

**Violation Example**: JSON contains `".cursor/rules/test.mdc"` instead of `"/home/user/project/.cursor/rules/test.mdc"`.

**Test Location**: `tests/contracts/paths.rs::contract_json_paths_absolute`

---

### PATH-005: Cross-Platform Path Separators

**Promise**: Lockfile keys and internal path representations use forward slashes (`/`) on all platforms.

**Violation Example**: On Windows, lockfile contains `project:C:\Users\foo\.cursor\rules\test.mdc`.

**Test Location**: `tests/contracts/paths.rs::contract_lockfile_paths_use_forward_slash`

---

## Scope Contracts

### SCOPE-001: Project Scope Isolation

**Promise**: Assets with `scope: project` only affect the project directory, never the home directory, even if `--home` is accidentally specified.

**Violation Example**: A project-scoped policy gets written to `~/.cursor/rules/`.

**Test Location**: `tests/contracts/scope.rs::contract_project_scope_stays_in_project`

---

### SCOPE-002: User Scope Home Location

**Promise**: Assets with `scope: user` are deployed to the home directory (or appropriate XDG location), not the project.

**Violation Example**: A user-scoped action appears in `./. claude/commands/` instead of `~/.claude/commands/`.

**Test Location**: `tests/contracts/scope.rs::contract_user_scope_uses_home`

---

### SCOPE-003: Orphan Detection Scope Boundary

**Promise**: Orphan detection only considers files in the current deployment scope. Switching from `--project` to `--home` does not mark project files as orphans.

**Violation Example**: User deploys with `--project`, then runs `--home`, and project files are flagged for deletion.

**Test Location**: `tests/contracts/scope.rs::contract_orphan_detection_respects_scope`

**Related Issues**: commit 189cc23

---

## Configuration Contracts

### CONFIG-001: Layer Priority Order

**Promise**: Configuration is merged in priority order: CLI flags > environment variables > project config > user config > defaults.

**Violation Example**: Setting `CALVIN_TARGETS=cursor` is ignored because project config has `targets.enabled = ["claude-code"]`.

**Test Location**: `tests/contracts/config.rs::contract_config_priority_order`

---

### CONFIG-002: Project Config Overrides User

**Promise**: When a project has `.promptpack/config.toml`, its settings take precedence over `~/.calvin/config.toml`.

**Violation Example**: User sets `security_mode = "strict"` globally, but project config has `security_mode = "permissive"`, yet strict mode is still enforced.

**Test Location**: `tests/contracts/config.rs::contract_project_overrides_user_config`

**Related Issues**: commit 813a370

---

### CONFIG-003: Environment Variable Typo Warning

**Promise**: If a Calvin environment variable contains an invalid value that resembles a valid option, a warning is shown with a suggestion.

**Violation Example**: `CALVIN_SECURITY_MODE=strct` silently falls back to default instead of warning about the typo.

**Test Location**: `tests/contracts/config.rs::contract_env_var_typo_warning`

**Related Issues**: Principle from GEMINI.md "Never fail silently"

---

### CONFIG-004: Empty Array Means Disable

**Promise**: An explicit empty array in configuration (e.g., `targets.enabled = []`) means "disable all", not "use defaults".

**Violation Example**: User sets `targets.enabled = []` to disable all targets, but all targets are still deployed.

**Test Location**: `tests/contracts/config.rs::contract_empty_array_means_disable`

**Related Issues**: commit targets-config-bug-2025-12-24

---

## Layer Contracts

### LAYER-001: Layer Merge Order

**Promise**: Layers are merged in order: user layer (lowest priority) → additional layers → project layer (highest priority). Higher priority wins on conflict.

**Violation Example**: User layer policy overrides project layer policy.

**Test Location**: `tests/contracts/layers.rs::contract_layer_merge_order`

---

### LAYER-002: Missing Layer Graceful Error

**Promise**: If a configured layer path doesn't exist, Calvin shows a clear error message with remediation steps, not a cryptic file-not-found error.

**Violation Example**: "Error: No such file or directory" with no context.

**Test Location**: `tests/contracts/layers.rs::contract_missing_layer_graceful_error`

---

### LAYER-003: Layer Provenance Tracking

**Promise**: The lockfile records which layer each deployed asset came from, enabling accurate orphan detection across layer changes.

**Violation Example**: User removes an asset from user layer, but because lockfile doesn't track layer source, the deployed file becomes orphaned without detection.

**Test Location**: `tests/contracts/layers.rs::contract_lockfile_tracks_provenance`

---

### LAYER-004: Asset Aggregation Across Layers

**Promise**: Assets with different IDs from all layers are aggregated—all unique assets are deployed, not just those from the highest priority layer.

**Violation Example**: User layer has `global-style.md`, project layer has `local-style.md`, but only `local-style.md` is deployed.

**Test Location**: `tests/contracts/layers.rs::contract_layer_merge_aggregates_unique_assets`

**Related Issues**: PRD §4.2

---

### LAYER-005: Override Information Tracked

**Promise**: When a higher-priority layer overrides a lower-priority asset, the lockfile records which layer was overridden (the `overrides` field).

**Violation Example**: Project overrides user's `code-style`, but lockfile doesn't record this, causing confusion in provenance reports.

**Test Location**: `tests/contracts/layers.rs::contract_lockfile_tracks_override_provenance`

**Related Issues**: PRD §10.2

---

### LAYER-006: `--no-user-layer` Excludes User Layer

**Promise**: The `--no-user-layer` CLI flag completely excludes user layer assets from the merge.

**Violation Example**: User runs `calvin deploy --no-user-layer` but user layer assets still appear in output.

**Test Location**: `tests/contracts/layers.rs::contract_no_user_layer_flag_excludes_user`

**Related Issues**: PRD §4.4

---

### LAYER-007: `--layer` Adds Additional Layer

**Promise**: The `--layer PATH` CLI flag adds an additional layer at the correct priority (after config additional layers, before project layer).

**Violation Example**: CLI-specified layer is ignored or applied at wrong priority in merge order.

**Test Location**: `tests/contracts/layers.rs::contract_layer_flag_adds_additional_layer`

**Related Issues**: PRD §4.4

---

### LAYER-008: Layer Migration No False Orphans

**Promise**: When an asset moves from one layer to another (same ID, same output path), the deployed file is NOT marked as orphan.

**Violation Example**: User removes `code-style.md` from project layer (user layer still has it), next deploy marks `.cursor/rules/code-style.mdc` as orphan.

**Test Location**: `tests/contracts/layers.rs::contract_layer_migration_no_false_orphan`

**Related Issues**: PRD §5.5

---

### LAYER-009: No Layers Found Shows Guidance

**Promise**: When no promptpack layers are found (no user, no additional, no project), Calvin shows a clear error with `calvin init --user` suggestion.

**Violation Example**: Silent failure or cryptic "cannot read directory" error.

**Test Location**: `tests/contracts/layers.rs::contract_no_layers_found_shows_guidance`

**Related Issues**: PRD §13.2

---

## Output Contracts

### OUTPUT-001: Signature in Generated Files

**Promise**: All Calvin-generated files contain a signature comment (e.g., `<!-- Generated by Calvin -->`) that identifies them as machine-generated.

**Violation Example**: A generated file has no signature, causing `clean` to skip it.

**Test Location**: `tests/contracts/output.rs::contract_generated_files_have_signature`

---

### OUTPUT-002: Clean Respects Signature

**Promise**: `calvin clean` only deletes files that have the Calvin signature, unless `--force` is used.

**Violation Example**: A user-edited file in `.cursor/rules/` is deleted because it was once deployed by Calvin.

**Test Location**: `tests/contracts/output.rs::contract_clean_respects_signature`

---

### OUTPUT-003: JSON Output is NDJSON

**Promise**: All `--json` output is valid Newline-Delimited JSON (one JSON object per line).

**Violation Example**: Multi-line JSON object breaks `jq` piping.

**Test Location**: `tests/contracts/output.rs::contract_json_is_ndjson`

---

### OUTPUT-004: Exit Codes

**Promise**: Exit codes follow standard conventions:

- 0: Success
- 1: General error (user-fixable)
- 2: Usage error (wrong arguments)

**Violation Example**: Validation failure exits with 0.

**Test Location**: `tests/contracts/output.rs::contract_exit_codes`

---

## Security Contracts

### SECURITY-001: Deny List Enforcement

**Promise**: Files matching the deny list patterns are never deployed, regardless of configuration.

**Violation Example**: A file named `.env.template` is deployed even though `.env*` is on the deny list.

**Test Location**: `tests/contracts/security.rs::contract_deny_list_enforced`

---

### SECURITY-002: Strict Mode Blocks Dangerous Patterns

**Promise**: In `security_mode = "strict"`, assets containing shell commands, file system operations, or other dangerous patterns are rejected.

**Violation Example**: An asset containing `rm -rf` is deployed in strict mode.

**Test Location**: `tests/contracts/security.rs::contract_strict_mode_blocks_dangerous`

---

## Interactive Contracts

### INTERACTIVE-001: No Prompt Without TTY

**Promise**: When stdin is not a TTY, Calvin never waits for user input. It either uses `--yes` behavior or fails gracefully.

**Violation Example**: CI pipeline hangs waiting for confirmation.

**Test Location**: `tests/contracts/interactive.rs::contract_no_prompt_without_tty`

---

### INTERACTIVE-002: Ctrl+C Restores Cursor

**Promise**: If the user presses Ctrl+C during an interactive prompt, the terminal cursor is restored before exit.

**Violation Example**: After Ctrl+C, terminal shows no cursor.

**Test Location**: `tests/contracts/interactive.rs::contract_ctrl_c_restores_cursor`

**Related Issues**: UI cursor restoration fix

---

## Adding New Contracts

When adding a new contract:

1. Assign the next number in the appropriate domain
2. Write the promise in user-facing terms
3. Provide a concrete violation example
4. Create the test before (or alongside) the fix
5. Link to related issues/commits

**Template**:

```markdown
### [DOMAIN]-[NUMBER]: [Short Name]

**Promise**: [What we guarantee]

**Violation Example**: [Concrete failure scenario]

**Test Location**: `tests/contracts/[file].rs::[test_name]`

**Related Issues**: [Links]
```

---

## Version History

| Date | Change |
|------|--------|
| 2025-12-25 | Initial contract registry |
