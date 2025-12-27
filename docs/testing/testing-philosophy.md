# Calvin Testing Philosophy

> The purpose of testing is not to prove the code works.  
> It is to guarantee the promises we make to users.

---

## Core Principle: Contract-First Testing

**We do not test implementations. We test contracts.**

A contract is a promise to the user that must hold regardless of:

- Which directory they run from
- What their HOME path looks like
- Whether files already exist
- What platform they're on
- How they invoke the command

### The Hierarchy of Test Value

```
                    ┌─────────────────────┐
                    │   User Scenarios    │  ← Highest value
                    │  (Real workflows)   │
                    ├─────────────────────┤
                    │     Contracts       │  ← Core guarantees
                    │  (Invariants)       │
                    ├─────────────────────┤
                    │    Integration      │  ← Module boundaries
                    │  (Composition)      │
                    ├─────────────────────┤
                    │       Unit          │  ← Lowest value
                    │   (Implementation)  │
                    └─────────────────────┘
```

Most codebases are bottom-heavy (many unit tests, few scenario tests).  
Calvin should be top-heavy.

---

## The Three Questions

Before writing any test, answer these three questions:

### 1. What promise does this test protect?

Bad: "It tests that `deploy` creates a `.cursor/rules/` file"  
Good: "It guarantees that deployed files are always placed relative to the target root"

### 2. What failure mode does this test prevent?

Bad: "It checks the function returns `Ok`"  
Good: "It prevents the lockfile from being written to the wrong directory when run from a subdirectory"

### 3. What user pain does this test eliminate?

Bad: "It verifies the JSON output format"  
Good: "It eliminates the pain of lockfile path corruption when switching between `--home` and `--project` modes"

If you cannot answer these clearly, **do not write the test yet**.

---

## Test Categories

### Category 1: Contracts (MUST HAVE)

**Location**: `tests/contracts/`

Contracts are invariants that must ALWAYS hold. A failing contract test is a P0 bug.

```rust
// tests/contracts/paths.rs

/// CONTRACT: Lockfile is always written relative to source root
/// 
/// This prevents the bug where running `calvin deploy` from a 
/// subdirectory causes the lockfile to be written to the wrong location.
#[test]
fn contract_lockfile_location_is_source_root() {
    // Test from multiple working directories
    // Test with --home and --project flags
    // Test with --source flag pointing elsewhere
}

/// CONTRACT: Displayed paths use tilde notation for HOME
/// 
/// This ensures user-facing output is readable and portable.
#[test]
fn contract_display_paths_use_tilde() {
    // All stdout/stderr output must not contain raw HOME path
    // Exception: --json output uses absolute paths
}
```

**Naming Convention**: `contract_<what_is_guaranteed>`

**Rules**:

- Contracts test across environments (different cwd, HOME, etc.)
- Contracts test across invocation methods (flags, config, env vars)
- Contracts document the "why" prominently in comments
- Contracts never test internal implementation details

### Category 2: Scenarios (SHOULD HAVE)

**Location**: `tests/scenarios/`

Scenarios test complete user workflows end-to-end.

```rust
// tests/scenarios/first_time_user.rs

/// SCENARIO: First-time user sets up Calvin
/// 
/// A user with no prior Calvin installation:
/// 1. Runs `calvin init --user`
/// 2. Adds a global prompt
/// 3. Deploys to a new project
/// 4. Verifies prompts appear in their editor
#[test]
fn scenario_first_time_user_setup() {
    // Simulate clean environment (no ~/.calvin)
    // Run actual CLI commands in sequence
    // Verify final state matches user expectation
}
```

**Naming Convention**: `scenario_<user_story>`

**Rules**:

- Scenarios run real CLI binary, not library functions
- Scenarios use realistic file content, not `"test content"`
- Scenarios verify the final state, not intermediate steps
- Scenarios are documented like user stories

### Category 3: Properties (SHOULD HAVE)

**Location**: `tests/properties/`

Properties use fuzzing/property-based testing to find edge cases.

```rust
// tests/properties/path_handling.rs

proptest! {
    /// PROPERTY: Any valid path can be normalized without error
    #[test]
    fn property_path_normalization_never_panics(
        path in any::<PathBuf>()
    ) {
        // Should never panic, only return Ok or Err
        let _ = normalize_path(&path);
    }
    
    /// PROPERTY: Tilde expansion is reversible
    #[test]
    fn property_tilde_round_trip(
        path in path_under_home()
    ) {
        let with_tilde = display_with_tilde(&path);
        let expanded = expand_tilde(&with_tilde);
        prop_assert_eq!(path, expanded);
    }
}
```

**Naming Convention**: `property_<invariant>`

**Rules**:

- Properties test mathematical invariants
- Properties use generators to explore input space
- Properties focus on "never fails" guarantees

### Category 4: Regression (AS NEEDED)

**Location**: `tests/regression/`

Regression tests capture specific bugs that were found and fixed.

```rust
// tests/regression/gh_42_lockfile_wrong_directory.rs

/// REGRESSION: GitHub Issue #42
/// 
/// Bug: Running `calvin deploy` from a subdirectory caused the 
/// lockfile to be written to the subdirectory instead of project root.
/// 
/// Fixed in: commit abc123
/// Related contract: contract_lockfile_location_is_source_root
#[test]
fn regression_gh_42_lockfile_in_subdirectory() {
    // Exact reproduction of the bug scenario
}
```

**Naming Convention**: `regression_<issue_id>_<short_description>`

**Rules**:

- Every bug fix MUST have a regression test
- Regression tests link to the issue and fix commit
- Regression tests reference related contract tests  
- Regression tests use the exact reproduction steps from the bug report

### Category 5: Snapshots (SPARINGLY)

**Location**: `tests/golden/`

Snapshots capture expected output for stable interfaces.

**Rules**:

- Only use for **stable, documented output formats**
- Never use for internal implementation details
- Review snapshot diffs carefully—don't blindly update
- Prefer contract tests over snapshots when possible

### Category 6: Unit Tests (MINIMAL)

**Location**: Inline in `src/` files

Unit tests are for complex pure functions only.

**Rules**:

- Only test functions with complex logic
- Prefer property tests over example-based unit tests
- Never test private implementation details
- If a function needs many unit tests, it's too complex—refactor it

---

## Test Environment Principles

### Principle 1: Environment Isolation

Every test must control its environment completely:

```rust
fn test_with_env(
    home: &Path,
    cwd: &Path,
    env_vars: &[(String, String)]
) -> TestResult {
    // Override HOME, XDG_*, cwd
    // Never leak to real user environment
}
```

### Principle 2: Environment Variation

High-value tests run across multiple environments:

```rust
#[test_case("project root", ".", expected_lockfile_at_root())]
#[test_case("subdirectory", "src/lib", expected_lockfile_at_root())]
#[test_case("sibling directory", "../other", expected_lockfile_at_root())]
fn deploy_lockfile_location(description: &str, cwd: &str, expectation: PathBuf) {
    // Same contract tested in multiple environments
}
```

### Principle 3: Real Paths, Not Mocks

Prefer real file system with temp directories over mocks:

```rust
// ❌ Bad: Mock everything
let fs = MockFileSystem::new();
fs.expect_write().returning(|_| Ok(()));

// ✅ Good: Real temp directory
let dir = tempdir()?;
deploy_to(dir.path())?;
assert!(dir.path().join("calvin.lock").exists());
```

Mocks hide bugs. Real file systems expose them.

---

## Test Quality Standards

### Standard 1: Descriptive Failure Messages

```rust
// ❌ Bad
assert!(result.is_ok());

// ✅ Good
assert!(
    result.is_ok(),
    "Deploy to {target:?} failed: {error:?}\n\
     CWD: {cwd:?}\n\
     HOME: {home:?}\n\
     Files in project: {files:?}"
);
```

### Standard 2: Minimal Setup, Maximal Clarity

```rust
// ❌ Bad: Wall of setup code
fn test_deploy() {
    let dir = tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".promptpack")).unwrap();
    std::fs::write(dir.path().join(".promptpack/config.toml"), "...").unwrap();
    std::fs::write(dir.path().join(".promptpack/test.md"), "...").unwrap();
    // ... 20 more lines of setup
    
    let output = Command::new(bin()).arg("deploy").output().unwrap();
    assert!(output.status.success());
}

// ✅ Good: Setup is named and reusable
fn test_deploy() {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_cursor_enabled()
        .build();
    
    let result = env.run(&["deploy"]);
    
    assert_deployed!(result, ".cursor/rules/test.mdc");
}
```

### Standard 3: Test the Edges, Not the Middle

```rust
// ❌ Bad: Only tests happy path
#[test]
fn deploy_works() { ... }

// ✅ Good: Tests boundaries
#[test]
fn deploy_to_readonly_directory_fails_gracefully() { ... }

#[test]
fn deploy_with_circular_includes_detected() { ... }

#[test]
fn deploy_when_target_file_modified_externally() { ... }
```

---

## Adding New Tests: Decision Tree

```
Is this a bug fix?
├── Yes → Add regression test in tests/regression/
│         Link to issue, commit, and related contract
└── No
    │
    Does this protect a user-facing promise?
    ├── Yes → Add contract test in tests/contracts/
    │         Test across environments and invocation methods
    └── No
        │
        Is this a complete user workflow?
        ├── Yes → Add scenario test in tests/scenarios/
        │         Use real CLI, realistic content
        └── No
            │
            Is this a mathematical invariant?
            ├── Yes → Add property test in tests/properties/
            │         Use proptest with generators
            └── No
                │
                Is this testing stable output format?
                ├── Yes → Add snapshot in tests/golden/
                │         Review diffs carefully
                └── No
                    │
                    Is this a complex pure function?
                    ├── Yes → Add unit test inline in src/
                    └── No → Do not write a test
                              (The code may need refactoring)
```

---

## Anti-Patterns

### Anti-Pattern 1: Testing Implementation Details

```rust
// ❌ This test will break when we refactor
#[test]
fn compiler_creates_intermediate_ast() {
    let compiler = Compiler::new();
    let ast = compiler.parse(input);  // Testing internal structure
    assert!(ast.nodes.len() == 3);
}
```

### Anti-Pattern 2: Overly Specific Assertions

```rust
// ❌ Brittle: breaks on any output format change
assert_eq!(output, "Deployed 3 files to .cursor/rules/\n");

// ✅ Robust: tests the contract, not the format
assert!(output.contains("3 files"));
assert!(output.contains(".cursor"));
```

### Anti-Pattern 3: Tests That Pass By Accident

```rust
// ❌ This passes because tempdir happens to be under /var
#[test]
fn deploy_creates_file() {
    let dir = tempdir().unwrap();
    deploy(dir.path());
    assert!(dir.path().join(".cursor").exists());
}

// ✅ Actually verifies the contract
#[test]
fn deploy_creates_file_at_correct_location() {
    let project_root = tempdir().unwrap();
    let deploy_target = project_root.path().join("nested/target");
    
    deploy_with_target(&deploy_target);
    
    assert!(
        deploy_target.join(".cursor").exists(),
        "Files should be at target, not project root"
    );
    assert!(
        !project_root.path().join(".cursor").exists(),
        "Files should NOT be at project root when target is specified"
    );
}
```

### Anti-Pattern 4: Testing Only Happy Paths

If all your tests pass on the first run, they're not testing enough.

---

## Metrics and Coverage

### What to Measure

| Metric | Target | Why |
|--------|--------|-----|
| Contract coverage | 100% of documented behaviors | Every promise must be tested |
| Regression coverage | 100% of fixed bugs | No bug should recur |
| Scenario coverage | All documented workflows | Users should never be surprised |
| Mutation score | >80% | Tests should fail when code breaks |

### What NOT to Obsess Over

| Metric | Don't target | Why |
|--------|--------------|-----|
| Line coverage | 90%+ | High coverage doesn't mean good tests |
| Number of tests | More is better | Quality over quantity |
| Test execution time | <1 second per test | Some tests need time to be thorough |

---

## Appendix: Test Infrastructure

### Required Crates

```toml
[dev-dependencies]
# Core testing
tempfile = "3"           # Isolated file system
insta = "1"              # Snapshot testing
proptest = "1"           # Property-based testing
test-case = "3"          # Parameterized tests

# Environment control
assert_cmd = "2"         # CLI testing
predicates = "3"         # Assertion helpers

# Optional
wiremock = "0.5"         # HTTP mocking (if needed)
```

### Test Helpers Location

```
tests/
├── common/              # Shared test utilities
│   ├── mod.rs
│   ├── env.rs           # TestEnv builder
│   ├── assertions.rs    # Custom assertions
│   └── fixtures.rs      # Fixture content constants
├── contracts/           # Invariant tests
├── scenarios/           # User workflow tests
├── properties/          # Property-based tests
├── regression/          # Bug reproduction tests
├── golden/              # Snapshot tests
└── integration.rs       # Test harness entry point
```

---

## Version History

| Date | Author | Change |
|------|--------|--------|
| 2025-12-25 | AI Assistant | Initial draft |

---

## References

- [Contract Testing (Martin Fowler)](https://martinfowler.com/bliki/ContractTest.html)
- [Property-Based Testing (Hypothesis)](https://hypothesis.readthedocs.io/en/latest/)
- [Test Desiderata (Kent Beck)](https://medium.com/@kentbeck_7670/test-desiderata-94150638a4b3)
