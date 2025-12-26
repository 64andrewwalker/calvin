# Multi-Layer PromptPack Test Plan

> Contract and Scenario tests for the multi-layer feature branch.
>
> Based on: `docs/archive/multi-layer-implementation/multi-layer-promptpack-prd.md`
> Aligned with: `docs/testing/TESTING_PHILOSOPHY.md`

---

## Design Principles Applied

This test plan follows the Testing Philosophy principles:

1. **Test Contracts, Not Implementations** - Each test protects a user-facing promise
2. **Environment Variation** - Tests run across different cwd, HOME configurations
3. **The Three Questions** - Every test answers: What promise? What failure mode? What user pain?

---

## Part 1: Contract Tests

### LAYER-MERGE: Layer Merge Semantics

These contracts ensure the layer merge behavior promised in PRD §4.2.

#### Contract: `LAYER-MERGE-001` - Priority Order

**Promise** (PRD §4.1): Layers are merged in priority order: user (lowest) → additional → project (highest). Higher priority assets completely override lower priority.

**Violation**: Project layer asset is ignored in favor of user layer asset.

```rust
#[test_case("user_only", vec!["user"], "user")]
#[test_case("project_overrides_user", vec!["user", "project"], "project")]
#[test_case("project_overrides_additional", vec!["user", "additional", "project"], "project")]
fn contract_layer_merge_priority(name: &str, layers: Vec<&str>, winner: &str) {
    let env = TestEnv::builder()
        .with_asset_in_layer("user", "shared.md", POLICY_FROM_USER)
        .with_asset_in_layer("additional", "shared.md", POLICY_FROM_ADDITIONAL)
        .with_asset_in_layer("project", "shared.md", POLICY_FROM_PROJECT)
        .with_active_layers(layers)
        .build();
    
    let result = env.run(&["deploy", "--yes"]);
    
    // Verify the deployed content matches the expected winner layer
    let deployed = env.read_deployed_file(".cursor/rules/shared.mdc");
    assert_contains!(deployed, format!("FROM_{}", winner.to_uppercase()));
}
```

#### Contract: `LAYER-MERGE-002` - Asset Aggregation

**Promise** (PRD §4.2): Assets with different IDs from all layers are aggregated (all included).

**Violation**: Assets from lower-priority layers are dropped even when they have unique IDs.

```rust
#[test]
fn contract_layer_merge_aggregates_unique_assets() {
    let env = TestEnv::builder()
        .with_user_asset("user-only.md", USER_POLICY)        // id: user-only
        .with_project_asset("project-only.md", PROJECT_POLICY) // id: project-only
        .build();
    
    let result = env.run(&["deploy", "--yes"]);
    
    // Both assets should be deployed
    assert_deployed!(env, ".cursor/rules/user-only.mdc");
    assert_deployed!(env, ".cursor/rules/project-only.mdc");
}
```

#### Contract: `LAYER-MERGE-003` - Override Information Tracked

**Promise** (PRD §10.2): When an asset is overridden, the lockfile records which layer was overridden.

**Violation**: Lockfile doesn't track override provenance, causing incorrect orphan detection.

```rust
#[test]
fn contract_lockfile_tracks_override_provenance() {
    let env = TestEnv::builder()
        .with_user_asset("shared.md", USER_POLICY)
        .with_project_asset("shared.md", PROJECT_POLICY)
        .build();
    
    env.run(&["deploy", "--yes"]);
    
    let lockfile = env.read_lockfile();
    let entry = lockfile.get_entry(".cursor/rules/shared.mdc");
    
    assert_eq!(entry.source_layer, "project");
    assert_eq!(entry.overrides, Some("user".to_string()));
}
```

---

### LAYER-CONFIG: Configuration Layer Handling

#### Contract: `LAYER-CONFIG-001` - Config Merge is Section-Level

**Promise** (PRD §14.2): Config merging is section-level replacement, not deep merge. Higher layer replaces entire section.

**Violation**: Deep merge causes unpredictable config behavior.

```rust
#[test]
fn contract_config_merge_replaces_sections() {
    let env = TestEnv::builder()
        .with_user_config(r#"
            [targets]
            enabled = ["claude-code", "cursor"]
            [security]
            mode = "balanced"
        "#)
        .with_project_config(r#"
            [targets]
            enabled = ["vscode"]
            # Note: no [security] section
        "#)
        .build();
    
    // Deploy should only target VSCode (project config replaces user's [targets])
    let result = env.run(&["deploy", "--yes"]);
    
    // VSCode output exists
    assert_deployed!(env, ".github/copilot-instructions.md");
    // Claude/Cursor outputs do NOT exist (overridden)
    assert_not_deployed!(env, ".claude/");
    assert_not_deployed!(env, ".cursor/");
}
```

#### Contract: `LAYER-CONFIG-002` - Empty Array Means Disable

**Promise** (PRD Config-004 in CONTRACT_REGISTRY): An explicit empty array means "disable all", not "use defaults".

**Violation**: Empty `targets.enabled = []` still deploys to all targets.

```rust
#[test]
fn contract_empty_config_array_means_disable() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(r#"
            [targets]
            enabled = []
        "#)
        .build();
    
    let result = env.run(&["deploy", "--yes"]);
    
    // No target directories should be created
    assert_not_deployed!(env, ".cursor/");
    assert_not_deployed!(env, ".claude/");
    assert_not_deployed!(env, ".vscode/");
}
```

---

### LAYER-PATH: Path Handling in Multi-Layer

#### Contract: `LAYER-PATH-001` - Lockfile at Project Root

**Promise** (PRD §9.2): Lockfile is always at project root (`./calvin.lock`), not in any layer's `.promptpack/`.

**Violation**: Lockfile is written to `.promptpack/.calvin.lock` or a layer's directory.

```rust
#[test_case(".", "from project root")]
#[test_case("src", "from subdirectory")]
#[test_case("src/lib/mod", "from deeply nested directory")]
fn contract_lockfile_at_project_root(cwd: &str, description: &str) {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_subdirectories(&["src", "src/lib", "src/lib/mod"])
        .build();
    
    let result = env.run_from(cwd, &["deploy", "--yes"]);
    
    // Lockfile must be at project root, regardless of cwd
    assert!(
        env.project_path("calvin.lock").exists() || 
        env.project_path(".promptpack/calvin.lock").exists(),
        "{}: Lockfile not at project root", description
    );
    
    // Lockfile must NOT be at cwd if cwd != project root
    if cwd != "." {
        assert!(
            !env.project_path(format!("{}/calvin.lock", cwd)).exists(),
            "{}: Lockfile should not be at cwd", description
        );
    }
}
```

#### Contract: `LAYER-PATH-002` - User Layer Path Uses Tilde

**Promise** (PRD §12, PATH-003): User-facing output shows paths with `~` notation.

**Violation**: Output shows `/Users/alice/.calvin` instead of `~/.calvin`.

```rust
#[test]
fn contract_user_layer_path_displays_with_tilde() {
    let env = TestEnv::builder()
        .with_user_layer_asset("global.md", GLOBAL_POLICY)
        .build();
    
    let result = env.run(&["layers"]);
    
    // Output should use tilde notation
    assert!(
        result.stdout.contains("~/.calvin") || result.stdout.contains("~\\"),
        "User layer path should use tilde notation.\nActual output:\n{}",
        result.stdout
    );
    
    // Should NOT contain raw home path
    assert_no_raw_home_path!(result, &env.home_dir);
}
```

#### Contract: `LAYER-PATH-003` - Source Layer Tracked in Lockfile

**Promise** (PRD §10.2): Each lockfile entry records `source_layer`, `source_asset`, `source_file`.

**Violation**: Provenance fields are missing, breaking orphan detection after layer changes.

```rust
#[test]
fn contract_lockfile_records_source_provenance() {
    let env = TestEnv::builder()
        .with_user_asset("from-user.md", POLICY_A)
        .with_project_asset("from-project.md", POLICY_B)
        .build();
    
    env.run(&["deploy", "--yes"]);
    
    let lockfile = env.read_lockfile();
    
    // User layer asset provenance
    let user_entry = lockfile.find_by_asset_id("from-user");
    assert_eq!(user_entry.source_layer, "user");
    assert!(user_entry.source_file.contains(".calvin/.promptpack"));
    
    // Project layer asset provenance
    let proj_entry = lockfile.find_by_asset_id("from-project");
    assert_eq!(proj_entry.source_layer, "project");
    assert!(proj_entry.source_file.contains(".promptpack"));
}
```

---

### LAYER-CLI: CLI Flag Behavior

#### Contract: `LAYER-CLI-001` - `--no-user-layer` Ignores User Layer

**Promise** (PRD §4.4): `--no-user-layer` flag excludes user layer from merge.

**Violation**: User layer assets are still deployed when flag is used.

```rust
#[test]
fn contract_no_user_layer_flag_excludes_user_assets() {
    let env = TestEnv::builder()
        .with_user_asset("global.md", GLOBAL_POLICY)      // id: global
        .with_project_asset("local.md", LOCAL_POLICY)     // id: local
        .build();
    
    let result = env.run(&["deploy", "--no-user-layer", "--yes"]);
    
    // Project asset deployed
    assert_deployed!(env, ".cursor/rules/local.mdc");
    // User asset NOT deployed
    assert_not_deployed!(env, ".cursor/rules/global.mdc");
}
```

#### Contract: `LAYER-CLI-002` - `--layer` Adds Additional Layer

**Promise** (PRD §4.4): `--layer PATH` adds an additional layer before project layer.

**Violation**: CLI-specified layer is ignored or applied at wrong priority.

```rust
#[test]
fn contract_layer_flag_adds_additional_layer() {
    let env = TestEnv::builder()
        .with_project_asset("project.md", PROJECT_POLICY)
        .build();
    
    let extra_layer = env.create_extra_layer_with_asset("extra.md", EXTRA_POLICY);
    
    let result = env.run(&[
        "deploy",
        "--layer", extra_layer.path_str(),
        "--yes"
    ]);
    
    // Both assets deployed
    assert_deployed!(env, ".cursor/rules/project.mdc");
    assert_deployed!(env, ".cursor/rules/extra.mdc");
}
```

#### Contract: `LAYER-CLI-003` - `--source` Replaces Project Layer

**Promise** (PRD §4.4): `--source PATH` replaces the project layer detection path.

**Violation**: Both default and specified source are used, or default is used when source specified.

```rust
#[test]
fn contract_source_flag_replaces_project_layer() {
    let env = TestEnv::builder()
        .with_project_asset("should-not-deploy.md", POLICY_A)
        .build();
    
    let alternate_source = env.create_promptpack_at("~/alt-source", "alt.md", POLICY_B);
    
    let result = env.run(&[
        "deploy",
        "--source", alternate_source.path_str(),
        "--yes"
    ]);
    
    // Alternate source asset deployed
    assert_deployed!(env, ".cursor/rules/alt.mdc");
    // Project layer asset NOT deployed
    assert_not_deployed!(env, ".cursor/rules/should-not-deploy.mdc");
}
```

---

### LAYER-ERROR: Error Handling

#### Contract: `LAYER-ERROR-001` - No Layers Found is Error

**Promise** (PRD §5.4, §13.3): When no layers exist (no user, no additional, no project), error with guidance.

**Violation**: Silent failure or cryptic error message.

```rust
#[test]
fn contract_no_layers_found_shows_guidance() {
    let env = TestEnv::builder()
        .without_user_layer()
        .without_project_promptpack()
        .build();
    
    let result = env.run(&["deploy"]);
    
    assert!(!result.success);
    assert_output_contains!(result, "no promptpack layers found");
    assert_output_contains!(result, "calvin init --user");  // Remediation suggestion
}
```

#### Contract: `LAYER-ERROR-002` - Missing Additional Layer Warns, Continues

**Promise** (PRD §5.4): If configured additional layer doesn't exist, warn but continue with other layers.

**Violation**: Hard error stops deployment, or missing layer silently ignored.

```rust
#[test]
fn contract_missing_additional_layer_warns_continues() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_home_config(r#"
            [sources]
            additional_layers = ["/nonexistent/path"]
        "#)
        .build();
    
    let result = env.run(&["deploy", "--yes"]);
    
    // Should succeed
    assert!(result.success, "Should continue with other layers");
    // Should warn about missing layer
    assert_output_contains!(result, "warning");
    assert_output_contains!(result, "/nonexistent/path");
    // Project asset should be deployed
    assert_deployed!(env, ".cursor/rules/test.mdc");
}
```

#### Contract: `LAYER-ERROR-003` - Duplicate Asset ID in Same Layer is Error

**Promise** (PRD §5.4): Same ID within a single layer is an error.

**Violation**: Silently picks one, causing data loss.

```rust
#[test]
fn contract_duplicate_id_same_layer_errors() {
    let env = TestEnv::builder()
        .with_project_asset("policies/style.md", policy_with_id("style"))
        .with_project_asset("actions/style.md", action_with_id("style"))  // Duplicate ID
        .build();
    
    let result = env.run(&["deploy"]);
    
    assert!(!result.success);
    assert_output_contains!(result, "duplicate");
    assert_output_contains!(result, "style");
}
```

---

### LAYER-ORPHAN: Orphan Detection with Layers

#### Contract: `LAYER-ORPHAN-001` - Layer Change Doesn't Create False Orphans

**Promise** (PRD §5.5): When asset moves from one layer to another, same output path doesn't become orphan.

**Violation**: Removing asset from project layer (while user layer has it) marks deployed file as orphan.

```rust
#[test]
fn contract_layer_migration_no_false_orphan() {
    let env = TestEnv::builder()
        .with_user_asset("shared.md", USER_POLICY)
        .with_project_asset("shared.md", PROJECT_POLICY)
        .build();
    
    // First deploy - project layer wins
    env.run(&["deploy", "--yes"]);
    
    // Remove from project layer (user layer still has it)
    env.remove_project_asset("shared.md");
    
    // Second deploy - user layer now provides asset
    let result = env.run(&["deploy", "--yes"]);
    
    // Should NOT report orphan (same output path, different source)
    assert!(!result.stdout.contains("orphan"));
    // File should still exist
    assert_deployed!(env, ".cursor/rules/shared.mdc");
}
```

#### Contract: `LAYER-ORPHAN-002` - Scope Change Respects Boundaries

**Promise** (SCOPE-003 in CONTRACT_REGISTRY): Switching scope doesn't mark other scope's files as orphans.

**Violation**: Deploy with --home, then --project marks home files as orphans.

```rust
#[test]
fn contract_scope_change_isolates_orphan_detection() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .build();
    
    // Deploy to home
    env.run(&["deploy", "--home", "--yes"]);
    assert_deployed_to_home!(env, ".cursor/rules/test.mdc");
    
    // Deploy to project (different scope)
    let result = env.run(&["deploy", "--project", "--yes"]);
    
    // Should NOT mark home files as orphans
    assert!(!result.stdout.contains("orphan"));
    // Home file should still exist (managed by different scope)
    assert_deployed_to_home!(env, ".cursor/rules/test.mdc");
}
```

---

## Part 2: Scenario Tests

### Scenario: First-Time User with Global Prompts

**Journey**: User with no Calvin setup wants to create reusable global prompts.

```rust
#[test]
fn scenario_first_time_user_global_prompts() {
    let env = TestEnv::builder()
        .fresh_environment()  // No ~/.calvin, no .promptpack
        .build();
    
    // Step 1: User runs calvin in empty project
    let result = env.run(&[]);
    assert_output_contains!(result, "Create");  // Guidance to create promptpack
    
    // Step 2: Initialize user layer
    let result = env.run(&["init", "--user"]);
    assert!(result.success);
    assert!(env.home_path(".calvin/.promptpack/config.toml").exists());
    
    // Step 3: User adds a global policy
    env.write_file(
        "~/.calvin/.promptpack/policies/my-style.md",
        PERSONAL_CODING_STYLE
    );
    
    // Step 4: Deploy in any project
    env.set_cwd("~/projects/webapp");
    env.ensure_git_repo();
    let result = env.run(&["deploy", "--yes"]);
    
    assert!(result.success);
    assert_deployed!(env, ".cursor/rules/my-style.mdc");
}
```

### Scenario: Team Shared Prompts with Project Override

**Journey**: Team shares prompts, project needs to override one.

```rust
#[test]
fn scenario_team_shared_with_project_override() {
    let env = TestEnv::builder()
        .with_home_config(r#"
            [sources]
            additional_layers = ["~/work/team-prompts/.promptpack"]
        "#)
        .build();
    
    // Team layer has shared policies
    env.create_team_layer_at("~/work/team-prompts/.promptpack")
        .with_asset("policies/code-style.md", TEAM_CODE_STYLE)
        .with_asset("policies/security.md", TEAM_SECURITY);
    
    // Project overrides code-style but keeps security
    env.with_project_asset("policies/code-style.md", PROJECT_CODE_STYLE);
    
    // Deploy
    let result = env.run(&["deploy", "-v", "--yes"]);
    
    // Verbose output shows override
    assert_output_contains!(result, "overrides");
    assert_output_contains!(result, "code-style");
    
    // Verify content
    let code_style = env.read_deployed_file(".cursor/rules/code-style.mdc");
    assert_contains!(code_style, "PROJECT");  // Project version deployed
    
    let security = env.read_deployed_file(".cursor/rules/security.mdc");
    assert_contains!(security, "TEAM");  // Team version deployed
}
```

### Scenario: Clean All Projects

**Journey**: User wants to remove Calvin from all projects at once.

```rust
#[test]
fn scenario_clean_all_projects() {
    let env = TestEnv::builder().build();
    
    // Setup: Deploy to multiple projects
    for project in ["proj-a", "proj-b", "proj-c"] {
        let project_dir = env.create_project(project);
        project_dir.with_promptpack_asset("test.md", SIMPLE_POLICY);
        env.run_in(project_dir.path(), &["deploy", "--yes"]);
    }
    
    // Verify registry
    let result = env.run(&["projects"]);
    assert_output_contains!(result, "proj-a");
    assert_output_contains!(result, "proj-b");
    assert_output_contains!(result, "proj-c");
    
    // Clean all with dry-run first
    let result = env.run(&["clean", "--all", "--dry-run"]);
    assert_output_contains!(result, "would delete");
    
    // Actually clean
    let result = env.run(&["clean", "--all", "--yes"]);
    
    // Verify cleaned
    for project in ["proj-a", "proj-b", "proj-c"] {
        assert!(!env.project_dir(project).join(".cursor").exists());
    }
}
```

### Scenario: Layer Visibility Check

**Journey**: User wants to see which layers are active and what each provides.

```rust
#[test]
fn scenario_layers_command_shows_full_stack() {
    let env = TestEnv::builder()
        .with_user_asset("global.md", GLOBAL_POLICY)
        .with_project_asset("local.md", LOCAL_POLICY)
        .build();
    
    let result = env.run(&["layers"]);
    
    assert!(result.success);
    
    // Shows layer stack with priority
    assert_output_contains!(result, "project");
    assert_output_contains!(result, "user");
    
    // Shows asset counts
    assert_output_contains!(result, "1 asset");  // or "assets"
    
    // Shows paths with tilde
    assert_output_contains!(result, "~/.calvin");
}
```

---

## Part 3: Property Tests

### Property: Layer Resolution Never Panics

```rust
proptest! {
    #[test]
    fn property_layer_resolution_never_panics(
        user_layer_exists in any::<bool>(),
        project_layer_exists in any::<bool>(),
        additional_count in 0..5usize,
    ) {
        let env = TestEnv::builder()
            .user_layer_exists(user_layer_exists)
            .project_layer_exists(project_layer_exists)
            .additional_layer_count(additional_count)
            .build();
        
        // Should never panic, only return Ok or graceful error
        let result = env.run(&["deploy", "--dry-run"]);
        
        // If no layers, should be clear error (not panic)
        if !user_layer_exists && !project_layer_exists && additional_count == 0 {
            prop_assert!(!result.success);
            prop_assert!(result.stderr.contains("no promptpack"));
        }
    }
}
```

### Property: Layer Path Normalization is Consistent

```rust
proptest! {
    #[test]
    fn property_layer_path_normalization_consistent(
        home_suffix in "[a-z/]{1,20}",
    ) {
        let env = TestEnv::builder().build();
        let layer_path = env.home_dir.join(&home_suffix);
        
        // Create layer
        env.create_layer_at(&layer_path);
        
        // Path should be normalized consistently in output
        let result = env.run(&["layers"]);
        
        // Should use tilde or consistent format
        let output = &result.stdout;
        
        // Count occurrences - path should appear consistently formatted
        let home_str = env.home_dir.to_string_lossy();
        prop_assert!(
            !output.contains(&*home_str) || output.contains(&format!("~/{}", home_suffix)),
            "Path should use consistent tilde notation"
        );
    }
}
```

---

## Implementation Priority

Based on bug frequency and user impact:

### P0: Must Have Before Release

| Contract ID | Risk if Missing |
|-------------|-----------------|
| `LAYER-MERGE-001` | Wrong asset wins, silent data corruption |
| `LAYER-PATH-001` | Lockfile in wrong location, breaks clean |
| `LAYER-ERROR-001` | User stuck with no guidance |
| `LAYER-ORPHAN-001` | False orphans deleted |

### P1: High Priority

| Contract ID | Risk if Missing |
|-------------|-----------------|
| `LAYER-MERGE-003` | Incorrect orphan detection |
| `LAYER-PATH-002` | Confusing output |
| `LAYER-PATH-003` | Broken provenance tracking |
| `LAYER-CLI-001/002/003` | CLI flags don't work |

### P2: Medium Priority

| Contract ID | Risk if Missing |
|-------------|-----------------|
| `LAYER-CONFIG-001/002` | Config merge surprises |
| `LAYER-ERROR-002/003` | Poor error experience |
| `LAYER-ORPHAN-002` | Edge case failures |

---

## Test File Structure

```
tests/
├── contracts/
│   ├── layer_merge.rs      # LAYER-MERGE-* tests
│   ├── layer_config.rs     # LAYER-CONFIG-* tests
│   ├── layer_path.rs       # LAYER-PATH-* tests
│   ├── layer_cli.rs        # LAYER-CLI-* tests
│   ├── layer_error.rs      # LAYER-ERROR-* tests
│   └── layer_orphan.rs     # LAYER-ORPHAN-* tests
├── scenarios/
│   └── multi_layer.rs      # All multi-layer scenarios
├── properties/
│   └── layer_resolution.rs # Property tests
└── common/
    ├── mod.rs
    ├── env.rs              # TestEnv builder
    ├── layer_helpers.rs    # Layer-specific helpers
    └── fixtures.rs         # Test content constants
```

---

## Version History

| Date | Author | Change |
|------|--------|--------|
| 2025-12-25 | AI Assistant | Initial multi-layer test plan |
