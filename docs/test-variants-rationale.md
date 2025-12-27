# Test Variants Rationale

This document explains the edge-case test variants added for skills support (deploy warnings + skill directory loading).

## Scope

These variants focus on two areas introduced around skills:

- Surfacing adapter diagnostics for generated `SKILL.md` outputs during `calvin deploy`
- Emitting a global warning when deploy targets include platforms that do not support skills (VS Code, Antigravity)
- Loading skill directories and handling supplemental files safely (hidden files/dirs, empty files, binary detection)

## Added Variant Coverage

### `deploy_warns_on_dangerous_allowed_tools_during_deploy`

- `deploy_warns_on_dangerous_allowed_tools_during_deploy__with_no_allowed_tools`: input boundary (no `allowed-tools`) → no false-positive warning.
- `deploy_warns_on_dangerous_allowed_tools_during_deploy__with_multiple_dangerous_tools`: input boundary (multiple dangerous tools) → multiple warnings emitted.
- `deploy_warns_on_dangerous_allowed_tools_during_deploy__when_skill_not_deployed_to_target`: state variation (skill not compiled for selected deploy target) → no deploy-time warning.

### `deploy_warns_when_deploy_targets_include_platforms_without_skill_support`

- `deploy_warns_when_deploy_targets_include_platforms_without_skill_support__without_skills`: state variation (no skills exist) → no spurious global warning.
- `deploy_warns_when_deploy_targets_include_platforms_without_skill_support__with_skill_targets_supported_only`: boundary (skill targets restricted to supported platforms) → no spurious global warning even with `--targets all`.
- `deploy_warns_when_deploy_targets_include_platforms_without_skill_support__with_single_unsupported_target`: off-by-one (single unsupported target selected) → warning includes only that platform.
- `deploy_warns_when_deploy_targets_include_platforms_without_skill_support__with_duplicate_deploy_targets`: type/collection edge (duplicates) → warning does not repeat platforms.
- `deploy_warns_when_deploy_targets_include_platforms_without_skill_support__with_no_deploy_targets`: uninitialized/disabled state (config disables all targets) → no warning.

### `test_load_skill_directory_valid`

- `test_load_skill_directory_valid__with_empty_supplemental_file`: input boundary (empty file) → accepted and preserved as an empty string.
- `test_load_skill_directory_valid__skips_hidden_files`: error-injection/security (hidden binary) → skipped before binary detection, load remains successful.
- `test_load_skill_directory_valid__skips_hidden_directories`: state variation (hidden directory present) → entire hidden subtree skipped.

### `test_load_skill_directory_binary_supplemental_rejected`

- `test_load_skill_directory_binary_supplemental_rejected__with_nul_in_text`: boundary/type edge (UTF-8 text containing `NUL`) → rejected as binary, with path included in error.

### `generate_skill_md_includes_frontmatter_body_footer`

- `generate_skill_md_includes_frontmatter_body_footer__with_colon_in_description`: input boundary (colon in scalar) → ensures generated SKILL.md frontmatter is valid YAML (quoted when needed).
- `generate_skill_md_includes_frontmatter_body_footer__with_allowed_tool_containing_colon_space`: input boundary (colon+space inside tool name) → ensures list items serialize as scalars, not YAML mappings.

### `compile_skill_outputs_rejects_parent_dir_supplemental_paths`

- `compile_skill_outputs_rejects_parent_dir_supplemental_paths__with_absolute_path`: error-injection/security (absolute path) → ensures adapters reject path traversal for supplementals.
- `compile_skill_outputs_rejects_parent_dir_supplemental_paths__with_windows_rooted_path`: platform edge (Windows rooted path without drive letter) → prevents `Path::join()` from ignoring the base skill dir.
- `compile_skill_outputs_rejects_parent_dir_supplemental_paths__with_windows_drive_path`: platform edge (Windows drive path) → prevents drive/prefix escape.

### `is_dangerous_skill_tool_flags_rm`

- `is_dangerous_skill_tool_flags_rm__with_args`: type/coercion edge (`"rm -rf"` tokenization) → still flagged as dangerous.

### `skill_root_from_path_returns_none_outside_skills`

- `skill_root_from_path_returns_none_outside_skills__when_path_is_skills_dir`: boundary (path is the `skills/` directory itself) → does not misclassify as a skill root.

### `target_supports_skills`

- `target_supports_skills__all_is_false`: boundary/meta-target (`Target::All`) → requires expansion before classification.

## Anti-Overfitting Measures

- Tests assert stable substrings (e.g., platform names and warning key phrases) rather than full CLI formatting.
- Variants use different target selections to exercise behavior without relying on ordering beyond what is contractually meaningful.

## Property-Based Test Candidates

- For any subset of dangerous tools in `allowed-tools`, deploy should warn once per dangerous tool and never warn for safe tools.
- For any deploy target set and any skill effective target set, the global "Skills skipped for" warning should appear iff the intersection is non-empty.

## Mutation Testing Recommendations

- Use `cargo-mutants` to validate that tests catch:
  - Removing the unsupported-target filter (VS Code / Antigravity) in the global warning.
  - Removing deduplication of deploy targets (duplicate target variant should fail).
  - Skipping adapter validation during deploy (dangerous-tool deploy variants should fail).
