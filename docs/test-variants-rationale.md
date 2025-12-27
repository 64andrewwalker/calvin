# Test Variants Rationale

This document explains the edge-case test variants added for the skills deploy warning logic.

## Scope

These variants focus on the deploy-time behavior introduced around skills:

- Surfacing adapter diagnostics for generated `SKILL.md` outputs during `calvin deploy`
- Emitting a global warning when deploy targets include platforms that do not support skills (VS Code, Antigravity)

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

