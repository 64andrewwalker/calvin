# Phase 5: Test Migration Checklist

> Goal: migrate legacy `tests/*.rs` CLI integration tests into the Contract/Scenario/Property structure and shared `TestEnv` helpers, while removing duplicated setup code.

## Status Legend

- [ ] Not started
- [~] In progress
- [x] Migrated / done
- [=] Keep as-is (no migration planned)

## Scope

**Legacy targets (to migrate):** most `tests/cli_*.rs` files (currently many repeat tempdir + HOME/XDG + promptpack setup).

**Already on the new framework (do not migrate):**
- `tests/contracts.rs` + `tests/contracts/*`
- `tests/scenarios.rs` + `tests/scenarios/*`
- `tests/properties.rs` + `tests/properties/*`

**Keep as-is (unless a future need arises):**
- `tests/integration.rs` + `tests/golden/*` (snapshot tests)
- `tests/no_hardcoded_doc_urls.rs`, `tests/no_hardcoded_ui_tokens.rs`, `tests/no_legacy_borders.rs` (meta guards)
- `tests/new_engine_integration.rs` (new engine integration)
- `tests/remote_tilde_expansion.rs` (remote tilde expansion regression/integration)

---

## P0: Deploy (highest ROI)

- [ ] `tests/cli_deploy_targets.rs` → split + migrate to `tests/contracts/*` (target selection) + `tests/regression/*` (cursor commands) + shared `TestEnv`
- [x] `tests/cli_deploy_orphans.rs` → migrated to shared `TestEnv` (still legacy crate; move/split later)
- [x] `tests/cli_deploy_cleanup.rs` → migrated to shared `TestEnv` (still legacy crate; merge/split later)
- [x] `tests/cli_deploy_source_flag.rs` → migrated to shared `TestEnv` (still legacy crate; move to PATH contracts later)
- [x] `tests/cli_deploy_source_external.rs` → migrated to shared `TestEnv` (still legacy crate; move to PATH contracts later)
- [x] `tests/cli_deploy_layers_flags.rs` → migrated to shared `TestEnv` (still legacy crate; move to LAYER contracts later)
- [x] `tests/cli_deploy_disable_project_layer.rs` → migrated to shared `TestEnv` (still legacy crate; move later)
- [x] `tests/cli_deploy_user_layer_targets_config.rs` → migrated to shared `TestEnv` (still legacy crate; move later)
- [x] `tests/cli_deploy_project_flag_overrides_home_config.rs` → migrated to shared `TestEnv` (still legacy crate; move later)
- [ ] `tests/cli_deploy_empty_project_targets_section_does_not_override_user_layer.rs` → migrate to CONFIG contracts + shared `TestEnv`
- [ ] `tests/cli_deploy_verbose_provenance.rs` → migrate to provenance contracts (or keep as scenario) + shared `TestEnv`
- [ ] `tests/cli_deploy_verbose_variants.rs` → migrate to scenario/contract as appropriate + shared `TestEnv`

## P0: Clean (highest ROI)

- [ ] `tests/cli_clean.rs` → split + migrate to `tests/contracts/output.rs` / `tests/contracts/clean.rs` + shared `TestEnv`
- [x] `tests/cli_clean_all.rs` → migrated to shared `TestEnv` (dedupe with scenario later)
- [x] `tests/cli_home_deploy_global_lockfile.rs` → migrated to shared `TestEnv` (still legacy crate; move to PATH contracts later)

---

## P1: Check / Diff / Layers / Projects

- [ ] `tests/cli_check.rs` → migrate to contracts/scenarios + shared `TestEnv`
- [ ] `tests/cli_check_all.rs` → migrate to scenarios + shared `TestEnv`
- [ ] `tests/cli_check_all_layers.rs` → migrate to scenarios + shared `TestEnv`
- [ ] `tests/cli_check_respects_user_layer_promptpack_security_config.rs` → migrate to CONFIG/SECURITY contracts + shared `TestEnv`
- [ ] `tests/cli_diff_includes_user_layer_assets.rs` → migrate to LAYER contracts + shared `TestEnv`
- [ ] `tests/cli_diff_source_external.rs` → migrate to PATH contracts + shared `TestEnv`
- [ ] `tests/cli_layers.rs` → migrate to PATH/LAYER contracts + shared `TestEnv`
- [ ] `tests/cli_layers_format.rs` → migrate to UI output contracts (or keep as snapshot) + shared `TestEnv`
- [ ] `tests/cli_layers_snapshot.rs` → likely replace with snapshot tests under `tests/golden/*`
- [ ] `tests/cli_projects.rs` → migrate to scenarios + shared `TestEnv`
- [ ] `tests/cli_provenance.rs` → migrate to provenance contracts + shared `TestEnv`

## P1: Init / Parse / Explain

- [ ] `tests/cli_init_user.rs` → migrate to scenarios + shared `TestEnv`
- [ ] `tests/cli_init_user_format.rs` → migrate to scenario/snapshot + shared `TestEnv`
- [ ] `tests/cli_init_project_config_template.rs` → migrate to scenarios + shared `TestEnv`
- [ ] `tests/cli_parse.rs` → migrate to parser contracts/properties + shared `TestEnv`
- [ ] `tests/cli_explain.rs` → migrate to scenarios + shared `TestEnv`

## P1: JSON output

- [ ] `tests/cli_json_deploy.rs` → migrate to OUTPUT contracts + shared `TestEnv`
- [ ] `tests/cli_json_check.rs` → migrate to OUTPUT contracts + shared `TestEnv`

## P1: Multi-layer

- [ ] `tests/cli_multi_layer.rs` → migrate to `tests/contracts/layers.rs` + scenarios + shared `TestEnv`
- [ ] `tests/cli_multi_layer_migration.rs` → migrate to regression + shared `TestEnv`
- [ ] `tests/cli_no_layers_found.rs` → migrate to LAYER error contracts + shared `TestEnv`

## P1: Interactive (potentially fragile; keep focused)

- [ ] `tests/cli_interactive_additional_layers_only.rs` → scenario/interactive contracts + shared `TestEnv`
- [ ] `tests/cli_interactive_layer_stack_display.rs` → scenario + shared `TestEnv`
- [ ] `tests/cli_interactive_legacy_user_config_path.rs` → regression/contract (legacy config path) + shared `TestEnv`
- [ ] `tests/cli_interactive_user_layer_empty.rs` → scenario/contract + shared `TestEnv`
- [ ] `tests/cli_interactive_user_layer_path_config.rs` → CONFIG contracts + shared `TestEnv`

## P1: Watch (potentially slow/flaky)

- [ ] `tests/cli_watch.rs` → evaluate: keep as integration or move behind feature flag
- [ ] `tests/cli_watch_source_external.rs` → evaluate: keep as integration or convert to contract
- [ ] `tests/cli_watch_includes_user_layer_assets.rs` → evaluate: keep as integration or convert to contract

---

## P2: Misc CLI

- [ ] `tests/cli_error_docs_links.rs` → migrate to docs/error contracts
- [ ] `tests/cli_registry_corrupted.rs` → migrate to regression/contract
- [ ] `tests/cli_lockfile_migration.rs` → migrate to regression/contract
- [ ] `tests/cli_lockfile_version_mismatch.rs` → migrate to regression/contract
- [ ] `tests/cli_migrate.rs` → migrate to scenarios
- [ ] `tests/cli_help.rs` → keep or migrate to snapshot (low value)
- [ ] `tests/cli_version.rs` → keep (low value)
- [ ] `tests/cli_yes.rs` → keep or merge into interactive contracts

## Non-CLI (evaluate case-by-case)

- [ ] `tests/config_promptpack_layer_merge_section_override.rs` → migrate to CONFIG contract test (layer merge semantics)
