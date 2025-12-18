# Deploy.rs Refactoring Plan

## Current Structure (God Object)

```
run_deploy() - 860 lines
├── Config loading (5 lines)
├── JSON output start (20 lines)
├── Header rendering (50 lines)
├── Security checks (150 lines)
├── DeployTarget::Project (145 lines)
│   ├── Scan + compile
│   ├── JSON mode sync
│   ├── Animation mode sync
│   └── Plain sync
├── DeployTarget::Home (150 lines)
│   ├── Same structure as Project
│   └── Different dist_root
├── DeployTarget::Remote (270 lines)  ← Most complex
│   ├── rsync fast path
│   ├── JSON mode SSH
│   ├── Animation mode SSH
│   ├── Interactive mode SSH
│   └── Plain rsync fallback
└── Result output (30 lines)
```

## Problems

1. **Code duplication**: Project/Home/Remote share 90% logic
2. **Mixed concerns**: UI, sync, conflict detection in one function
3. **Nested conditions**: 5+ levels deep in Remote branch
4. **Hard to test**: No unit tests for run_deploy()

## Target Structure

```
deploy/
├── mod.rs           - Public API (cmd_deploy, cmd_install, cmd_sync)
├── runner.rs        - DeployRunner struct with step methods  
├── targets.rs       - DeployTarget, ScopePolicy enums
├── ui.rs            - Header, progress, summary rendering
└── sync.rs          - Unified sync using two-stage API
```

## New Design

### 1. DeployRunner Struct

```rust
pub struct DeployRunner {
    source: PathBuf,
    target: DeployTarget,
    options: DeployOptions,
    ui: UiContext,
}

impl DeployRunner {
    pub fn new(...) -> Self;
    
    // Step methods (each ~50 lines)
    fn load_config(&mut self) -> Result<Config>;
    fn scan_assets(&self) -> Result<Vec<PromptAsset>>;
    fn compile_outputs(&self, assets: &[PromptAsset]) -> Result<Vec<OutputFile>>;
    fn check_security(&self, outputs: &[OutputFile]) -> Result<()>;
    fn sync_outputs(&self, outputs: &[OutputFile]) -> Result<SyncResult>;
    fn render_result(&self, result: &SyncResult);
    
    // Main entry
    pub fn run(&mut self) -> Result<()> {
        let config = self.load_config()?;
        let assets = self.scan_assets()?;
        let outputs = self.compile_outputs(&assets)?;
        self.check_security(&outputs)?;
        let result = self.sync_outputs(&outputs)?;
        self.render_result(&result);
        Ok(())
    }
}
```

### 2. Unified Sync Using Two-Stage API

```rust
fn sync_outputs(&self, outputs: &[OutputFile]) -> Result<SyncResult> {
    let dest = self.target.to_sync_destination();
    let lockfile = self.load_lockfile()?;
    
    // Stage 1: Plan
    let plan = plan_sync(outputs, &dest, &lockfile, &self.fs())?;
    
    // Stage 2: Resolve conflicts
    let final_plan = if self.options.force || plan.conflicts.is_empty() {
        plan.overwrite_all()
    } else if self.options.interactive {
        let (p, status) = resolve_conflicts_interactive(plan, &dest, &self.fs());
        if status == ResolveResult::Aborted {
            return Err(anyhow!("Aborted"));
        }
        p
    } else {
        plan.skip_all()
    };
    
    // Stage 3: Execute (auto-selects rsync for batch)
    execute_sync(&final_plan, &dest, SyncStrategy::Auto)
}
```

### 3. DeployTarget with SyncDestination

```rust
impl DeployTarget {
    fn to_sync_destination(&self) -> SyncDestination {
        match self {
            DeployTarget::Project => SyncDestination::Local(project_root),
            DeployTarget::Home => SyncDestination::Local(home_dir()),
            DeployTarget::Remote(s) => {
                let (host, path) = parse_remote(s);
                SyncDestination::Remote { host, path }
            }
        }
    }
}
```

## Implementation Steps

### Phase 1: Extract (current)
- [x] Create sync::plan module
- [x] Create sync::execute module
- [x] Create commands::sync_runner helper

### Phase 2: Refactor deploy.rs
- [ ] Create deploy/targets.rs - move enums
- [ ] Create deploy/runner.rs - move run_deploy
- [ ] Simplify run_deploy to use sync_runner
- [ ] Remove duplicated Project/Home/Remote branches

### Phase 3: Test
- [ ] Add unit tests for DeployRunner
- [ ] Integration tests for two-stage sync
- [ ] Performance benchmark (rsync vs SSH)

## Expected Outcome

- `deploy.rs`: ~200 lines (down from 1000)
- `deploy/runner.rs`: ~300 lines (well-structured)
- All three targets use same sync path
- Interactive -> rsync after "overwrite all"
