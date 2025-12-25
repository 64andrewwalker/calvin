# Calvin Testing Framework Implementation Plan

> From philosophy to practice: how to build the testing infrastructure.

---

## TODO Tracker

> **Status Legend**: ⬜ Not Started | 🟡 In Progress | ✅ Complete

### Phase 0: Foundation

- [x] Create `tests/common/mod.rs` with public exports
- [x] Implement `TestEnv` struct with temp dir management
- [x] Implement `TestEnvBuilder` with fluent API
- [x] Create `assert_deployed!` macro
- [x] Create `assert_output_contains!` macro
- [x] Create `assert_no_raw_home_path!` macro
- [x] Add fixture constants (`SIMPLE_POLICY`, `USER_POLICY`, etc.)
- [ ] Migrate 5+ existing tests to use `TestEnv`

### Phase 1: Contract Tests

- [x] Create `tests/contracts/` directory structure
- [x] Implement PATH contracts:
  - [x] `contract_lockfile_at_source_root` (partial - found bug, see FIXME)
  - [x] `contract_deployed_files_at_target` (partial - found bug, see FIXME)
  - [x] `contract_display_paths_use_tilde`
  - [x] `contract_json_paths_absolute`
- [x] Implement LAYER contracts:
  - [x] `contract_layer_merge_priority`
  - [x] `contract_layer_merge_aggregates_unique_assets`
  - [ ] `contract_lockfile_tracks_provenance`
  - [x] `contract_no_user_layer_flag_excludes_user`
  - [x] `contract_layer_migration_no_false_orphan`
- [ ] Implement CONFIG contracts:
  - [ ] `contract_config_priority_order`
  - [ ] `contract_empty_array_means_disable`

### Phase 2: Scenario Tests

- [ ] Create `tests/scenarios/` directory
- [ ] Document user journeys in `docs/testing/journeys/`
- [ ] Implement `scenario_first_time_user_setup`
- [ ] Implement `scenario_team_shared_with_override`
- [ ] Implement `scenario_clean_all_projects`

### Phase 3: Property Tests

- [ ] Add `proptest` to dev-dependencies
- [ ] Create `tests/properties/` directory
- [ ] Implement `property_normalize_never_panics`
- [ ] Implement `property_tilde_round_trip`
- [ ] Implement `property_layer_resolution_never_panics`

### Phase 4: CI Integration

- [ ] Create `.github/workflows/test-matrix.yml`
- [ ] Add contract tests to pre-commit hook
- [ ] Configure cross-platform test matrix (Linux, macOS, Windows)

### Phase 5: Migration

- [ ] Audit all 60+ existing test files
- [ ] Create migration checklist for each file
- [ ] Migrate high-value tests first (cli_deploy_*, cli_clean_*)
- [ ] Remove duplicate helper code
- [ ] Achieve 50% migration coverage

---

### Bugs Discovered by Contract Tests

> These bugs were found during contract test implementation and should be fixed.

| Bug ID | Contract | Description | Status |
|--------|----------|-------------|--------|
| PATH-BUG-001 | PATH-001 | `--source` flag doesn't affect lockfile location | 🔴 Open |
| PATH-BUG-002 | PATH-002 | `--source` flag doesn't affect deploy target location | 🔴 Open |
| OUTPUT-BUG-001 | OUTPUT-004 | Exit code 0 returned even when errors occur | 🔴 Open |

---

### Progress Summary

| Phase | Status | Completion |
|-------|--------|------------|
| Phase 0: Foundation | 🟡 In Progress | 7/8 |
| Phase 1: Contracts | 🟡 In Progress | 8/11 |
| Phase 2: Scenarios | ⬜ Not Started | 0/5 |
| Phase 3: Properties | ⬜ Not Started | 0/5 |
| Phase 4: CI | ⬜ Not Started | 0/3 |
| Phase 5: Migration | ⬜ Not Started | 0/5 |

**Last Updated**: 2025-12-25

---

## Phase 0: Foundation (Week 1)

### 0.1 Test Infrastructure Setup

**Goal**: Create shared test utilities that make writing contract tests easy.

```
tests/
├── common/
│   ├── mod.rs           # Public exports
│   ├── env.rs           # TestEnv builder (see below)
│   ├── assertions.rs    # Custom assert macros
│   ├── fixtures.rs      # Reusable content fixtures
│   └── capture.rs       # Output capture utilities
```

#### TestEnv Builder

The `TestEnv` builder is the core abstraction. It must:

1. Create isolated temp directories for project and home
2. Set environment variables (HOME, XDG_CONFIG_HOME, etc.)
3. Provide helper methods for common setups
4. Clean up automatically on drop

```rust
// tests/common/env.rs

pub struct TestEnv {
    pub project_root: TempDir,
    pub home_dir: TempDir,
    env_backup: HashMap<String, Option<String>>,
}

impl TestEnv {
    pub fn builder() -> TestEnvBuilder { ... }
    
    /// Run calvin CLI in this environment
    pub fn run(&self, args: &[&str]) -> TestResult { ... }
    
    /// Run from a specific subdirectory
    pub fn run_from(&self, cwd: &Path, args: &[&str]) -> TestResult { ... }
    
    /// Get path relative to project root
    pub fn project_path(&self, relative: &str) -> PathBuf { ... }
    
    /// Get path relative to home
    pub fn home_path(&self, relative: &str) -> PathBuf { ... }
}

pub struct TestEnvBuilder {
    project_assets: Vec<(String, String)>,
    user_layer_assets: Vec<(String, String)>,
    project_config: Option<String>,
    home_config: Option<String>,
    targets: Vec<String>,
}

impl TestEnvBuilder {
    pub fn with_project_asset(self, name: &str, content: &str) -> Self { ... }
    pub fn with_user_layer_asset(self, name: &str, content: &str) -> Self { ... }
    pub fn with_project_config(self, toml: &str) -> Self { ... }
    pub fn with_targets(self, targets: &[&str]) -> Self { ... }
    pub fn build(self) -> TestEnv { ... }
}
```

#### Custom Assertions

```rust
// tests/common/assertions.rs

/// Assert that a file was deployed to the expected location
#[macro_export]
macro_rules! assert_deployed {
    ($env:expr, $path:expr) => {
        let full_path = $env.project_path($path);
        assert!(
            full_path.exists(),
            "Expected file at {}, but it doesn't exist.\n\
             Project root: {:?}\n\
             Files found: {:?}",
            $path,
            $env.project_root.path(),
            list_all_files($env.project_root.path())
        );
    };
}

/// Assert that output contains expected pattern
#[macro_export]
macro_rules! assert_output_contains {
    ($result:expr, $pattern:expr) => {
        assert!(
            $result.stdout.contains($pattern) || $result.stderr.contains($pattern),
            "Expected output to contain '{}'\nstdout: {}\nstderr: {}",
            $pattern, $result.stdout, $result.stderr
        );
    };
}

/// Assert that no raw HOME path appears in output
#[macro_export]
macro_rules! assert_no_raw_home_path {
    ($result:expr, $home:expr) => {
        let home_str = $home.to_string_lossy();
        assert!(
            !$result.stdout.contains(&*home_str),
            "Raw HOME path leaked to stdout.\n\
             HOME: {}\n\
             stdout: {}",
            home_str, $result.stdout
        );
    };
}
```

### 0.2 Migrate Existing Helpers

Audit existing tests and extract common patterns:

| Current Location | Migrate To | Examples |
|------------------|------------|----------|
| `create_test_project()` in various files | `TestEnvBuilder::basic_project()` | cli_deploy_targets.rs |
| `run_deploy()` helpers | `TestEnv::run()` | cli_clean.rs |
| Inline assertion checks | `assert_deployed!` macro | Multiple files |

---

## Phase 1: Contract Tests (Week 2-3)

### 1.1 Identify Core Contracts

Review all recent bugs and extract the underlying contracts they violated:

| Bug | Contract | Test Name |
|-----|----------|-----------|
| Lockfile in wrong directory | Lockfile is always at source root | `contract_lockfile_at_source_root` |
| Raw HOME in output | Display paths use tilde | `contract_display_paths_use_tilde` |
| --home with no user layer | Graceful error on missing layer | `contract_missing_layer_graceful` |
| Orphan detection false positives | Orphans only from changed scope | `contract_orphan_scope_isolation` |

### 1.2 Define Contract Test Structure

Each contract test file follows this structure:

```rust
// tests/contracts/lockfile.rs

//! Contracts for lockfile behavior
//!
//! These invariants must ALWAYS hold. Any failure is a P0 bug.

use common::*;

/// CONTRACT: Lockfile is always written to the source root directory
/// 
/// Context: Users may run `calvin deploy` from subdirectories.
/// The lockfile must always be at the promptpack source location,
/// not at the current working directory.
mod lockfile_location {
    use super::*;
    
    #[test_case(".", "from project root")]
    #[test_case("src", "from src subdirectory")]
    #[test_case("src/lib", "from nested subdirectory")]
    fn contract_lockfile_at_source_root(cwd: &str, description: &str) {
        let env = TestEnv::builder()
            .with_project_asset("test.md", SIMPLE_POLICY)
            .build();
        
        let result = env.run_from(
            &env.project_path(cwd),
            &["deploy", "--yes"]
        );
        
        assert!(result.success, "{}: deploy failed - {}", description, result.stderr);
        
        // Lockfile must be at project root, not cwd
        let lockfile_at_root = env.project_path(".promptpack/.calvin.lock");
        assert!(
            lockfile_at_root.exists(),
            "{}: Lockfile should be at {:?}, not in subdirectory",
            description, lockfile_at_root
        );
    }
}

/// CONTRACT: Lockfile paths are portable across machines
/// 
/// Context: Users may share projects between machines with different
/// usernames/home directories. Lockfile paths should be relative or
/// use portable notation.
mod lockfile_portability {
    // ...
}
```

### 1.3 Priority Order

Implement contracts in priority order based on bug frequency:

1. **Path handling contracts** (highest bug frequency)
   - `contract_lockfile_at_source_root`
   - `contract_deployed_files_at_target`
   - `contract_display_paths_use_tilde`
   - `contract_json_paths_are_absolute`

2. **Scope contracts**
   - `contract_project_scope_stays_in_project`
   - `contract_user_scope_uses_home`
   - `contract_orphan_detection_respects_scope`

3. **Configuration contracts**
   - `contract_project_config_overrides_user`
   - `contract_cli_flags_override_config`
   - `contract_env_vars_warn_on_typo`

---

## Phase 2: Scenario Tests (Week 4)

### 2.1 Document User Journeys

Create explicit user journey documents before writing tests:

```markdown
# Journey: First-Time User

## Persona
Developer new to Calvin, wants to share prompts with team.

## Steps
1. Has a project with no .promptpack/
2. Runs `calvin` (interactive mode)
3. Sees "No promptpack found" message with helpful guidance
4. Runs `calvin init` to create .promptpack/
5. Creates a policy file
6. Runs `calvin deploy`
7. Opens Cursor, sees the rule applied

## Success Criteria
- Clear guidance at each step
- No confusing error messages
- Final state: working prompts in editor
```

### 2.2 Scenario Test Template

```rust
// tests/scenarios/first_time_user.rs

//! Scenario: First-time user sets up Calvin
//! 
//! Journey document: docs/testing/journeys/first-time-user.md

use common::*;

#[test]
fn scenario_first_time_user_complete_journey() {
    // Step 1: Clean environment
    let env = TestEnv::builder()
        .without_project_promptpack()
        .without_user_layer()
        .build();
    
    // Step 2: Interactive mode shows guidance
    let result = env.run(&[]);
    assert_output_contains!(result, "Create a new .promptpack");
    
    // Step 3: Init creates structure
    let result = env.run(&["init"]);
    assert!(result.success);
    assert!(env.project_path(".promptpack/config.toml").exists());
    
    // Step 4: User adds policy (simulated)
    env.write_file(".promptpack/policies/style.md", SIMPLE_POLICY);
    
    // Step 5: Deploy works
    let result = env.run(&["deploy", "--yes"]);
    assert!(result.success);
    assert_deployed!(env, ".cursor/rules/style.mdc");
    
    // Step 6: Check verifies
    let result = env.run(&["check"]);
    assert!(result.success);
    assert_output_contains!(result, "1 asset");
}
```

---

## Phase 3: Property Tests (Week 5)

### 3.1 Identify Testable Properties

Focus on functions with complex input handling:

| Function | Property |
|----------|----------|
| `normalize_path` | Never panics on any input |
| `display_with_tilde` | Round-trips with `expand_tilde` |
| `parse_frontmatter` | Returns Err on invalid YAML, never panics |
| `merge_configs` | Associative: merge(a, merge(b, c)) == merge(merge(a, b), c) |

### 3.2 Property Test Template

```rust
// tests/properties/paths.rs

use proptest::prelude::*;

proptest! {
    /// Any path can be normalized without panicking
    #[test]
    fn property_normalize_never_panics(
        path in any::<String>()
    ) {
        let _ = normalize_path(&PathBuf::from(path));
        // Just checking it doesn't panic
    }
    
    /// Tilde expansion and contraction are inverses
    #[test]
    fn property_tilde_round_trip(
        // Generate paths that would be under home
        suffix in "[a-z/]{0,50}"  
    ) {
        let home = dirs::home_dir().unwrap();
        let path = home.join(&suffix);
        
        let with_tilde = display_with_tilde(&path);
        let expanded = expand_tilde(&with_tilde);
        
        prop_assert_eq!(path, expanded);
    }
}
```

---

## Phase 4: CI Integration (Week 6)

### 4.1 Test Matrix

```yaml
# .github/workflows/test-matrix.yml
name: Test Matrix

on: [push, pull_request]

jobs:
  contracts:
    name: Contract Tests
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test contracts

  scenarios:
    name: Scenario Tests
    runs-on: ubuntu-latest
    strategy:
      matrix:
        scenario:
          - name: fresh-install
            env: {}
          - name: existing-user-layer
            env: { SETUP_USER_LAYER: "true" }
          - name: unicode-paths
            env: { PROJECT_NAME: "项目测试" }
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test scenarios

  properties:
    name: Property Tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test properties -- --test-threads=1
```

### 4.2 Pre-Commit Hook Update

```bash
# .git/hooks/pre-commit (addition)

# Run contract tests on files changed
if git diff --cached --name-only | grep -q "^src/"; then
    echo "Running contract tests..."
    cargo test --test contracts --quiet || exit 1
fi
```

---

## Phase 5: Migration (Week 7-8)

### 5.1 Categorize Existing Tests

Review all 60+ existing test files:

| Current File | New Location | Action |
|--------------|--------------|--------|
| cli_deploy_targets.rs | contracts/deploy_target.rs | Refactor |
| cli_clean.rs | contracts/clean.rs + scenarios/clean.rs | Split |
| golden/mod.rs | golden/mod.rs | Keep (stable) |
| cli_version.rs | Keep inline | Minimal value |
| cli_help.rs | Keep inline | Minimal value |

### 5.2 Migration Checklist

For each migrated test:

- [ ] Uses `TestEnv` builder
- [ ] Has clear contract/scenario documentation
- [ ] Tests across environments (if contract)
- [ ] Has descriptive failure messages
- [ ] Removes duplicate setup code

---

## Success Metrics

### Phase Completion Criteria

| Phase | Completion Criteria |
|-------|---------------------|
| 0 | `TestEnv` builder used in 5+ tests |
| 1 | 10+ contract tests covering path handling |
| 2 | 3+ scenario tests for major workflows |
| 3 | Property tests for all path functions |
| 4 | CI runs all test categories |
| 5 | 50% of old tests migrated |

### Long-Term Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Bug recurrence | 0% | No fixed bug should recur |
| Contract coverage | 100% | Every documented behavior has a contract test |
| Mean time to test | <5 min | New contributor can write a test in <5 min |

---

## Appendix: File Templates

### Contract Test Template

```rust
//! contracts/<domain>.rs - [Domain] contracts

use crate::common::*;

/// CONTRACT: [What is guaranteed]
/// 
/// Context: [Why this matters to users]
/// 
/// Related bugs: [GitHub issue links]
mod [contract_name] {
    use super::*;
    
    #[test_case(/* variants */)]
    fn contract_[name](/* params */) {
        let env = TestEnv::builder()
            // setup
            .build();
        
        let result = env.run(&[/* args */]);
        
        // assertions with descriptive messages
    }
}
```

### Scenario Test Template

```rust
//! scenarios/<journey>.rs - [Journey Name]
//!
//! Journey document: docs/testing/journeys/[journey].md

use crate::common::*;

/// SCENARIO: [User story summary]
#[test]
fn scenario_[name]() {
    // Step 1: [description]
    let env = TestEnv::builder().build();
    
    // Step 2: [description]  
    let result = env.run(&[/* args */]);
    assert!(/* condition */, "Step 2 failed: ...");
    
    // Final verification
}
```
