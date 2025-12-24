# Phase 4: Visibility & Tooling

> **Priority**: Low (Polish)  
> **Estimated Effort**: 1-2 days  
> **Dependencies**: Phase 1-3 complete

## Objective

添加可视化命令和工具，帮助用户理解和调试多层系统。

## Detailed Tasks

### Task 4.1: Implement `calvin layers` Command

**File**: `src/commands/layers.rs`

```rust
pub fn run_layers(json: bool) -> Result<()> {
    let config = Config::load_or_default(Some(&std::env::current_dir()?));
    let project_root = std::env::current_dir()?;
    
    let resolver = LayerResolver::from_config(&config, &project_root);
    let layers = resolver.resolve()?;
    
    if json {
        let output = serde_json::to_string_pretty(&layers)?;
        println!("{}", output);
    } else {
        render_layers_view(&layers);
    }
    
    Ok(())
}

fn render_layers_view(layers: &[Layer]) {
    println!("╭─────────────────────────────────────────────────────────────────╮");
    println!("│  📚 Layer Stack                                                 │");
    println!("╰─────────────────────────────────────────────────────────────────╯");
    println!();
    println!("Priority: high → low");
    println!("┌───────────────────────────────────────────────────────────────┐");
    
    for (i, layer) in layers.iter().enumerate().rev() {
        let priority = layers.len() - i;
        let layer_type = match layer.layer_type {
            LayerType::User => "[user]   ",
            LayerType::Custom => "[custom] ",
            LayerType::Project => "[project]",
        };
        println!(
            "│  {}. {} {:<35} {:>3} assets │",
            priority,
            layer_type,
            truncate(&layer.path.display().to_string(), 35),
            layer.assets.len()
        );
    }
    
    println!("└───────────────────────────────────────────────────────────────┘");
    println!();
    
    // 统计
    let total_assets: usize = layers.iter().map(|l| l.assets.len()).sum();
    let merged = merge_layers(layers);
    let overridden = total_assets - merged.assets.len();
    
    println!("Merged: {} assets ({} overridden)", merged.assets.len(), overridden);
}
```

**CLI Integration**:
```rust
#[derive(Subcommand)]
pub enum Commands {
    /// Show the layer stack
    Layers {
        #[arg(long)]
        json: bool,
    },
    // ...
}
```

### Task 4.2: Implement `calvin provenance` Command

**File**: `src/commands/provenance.rs`

```rust
pub fn run_provenance(json: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let lockfile_path = project_root.join("calvin.lock");
    
    if !lockfile_path.exists() {
        return Err(CalvinError::DirectoryNotFound { 
            path: lockfile_path 
        });
    }
    
    let lockfile = lockfile_repo.load(&lockfile_path)?;
    
    if json {
        let output = serde_json::to_string_pretty(&lockfile)?;
        println!("{}", output);
    } else {
        render_provenance_report(&lockfile);
    }
    
    Ok(())
}

fn render_provenance_report(lockfile: &Lockfile) {
    println!("╭─────────────────────────────────────────────────────────────────╮");
    println!("│  📋 Output Provenance Report                                    │");
    println!("╰─────────────────────────────────────────────────────────────────╯");
    println!();
    
    let entries: Vec<_> = lockfile.entries().collect();
    
    for (key, entry) in entries {
        // 解析 key 获取输出路径
        let (_, path) = Lockfile::parse_key(key).unwrap_or((Scope::Project, key));
        
        println!("{}", path);
        
        if let Some(source_file) = entry.source_file() {
            println!("├─ Source: {}", source_file.display());
        }
        if let Some(layer) = entry.source_layer() {
            println!("├─ Layer:  {}", layer);
        }
        if let Some(asset) = entry.source_asset() {
            println!("├─ Asset:  {}", asset);
        }
        if let Some(overrides) = entry.overrides() {
            println!("└─ Note:   Overrides '{}' from {} layer", 
                entry.source_asset().unwrap_or("?"), overrides);
        } else {
            println!("└─");
        }
        println!();
    }
    
    // 统计
    let layers: HashSet<_> = entries.iter()
        .filter_map(|(_, e)| e.source_layer())
        .collect();
    
    println!("Total: {} output files from {} layers", entries.len(), layers.len());
}
```

### Task 4.3: Update `calvin check` for Multi-Layer

**File**: `src/commands/check.rs`

```rust
pub fn run_check(all_layers: bool) -> Result<()> {
    let config = Config::load_or_default(Some(&std::env::current_dir()?));
    let project_root = std::env::current_dir()?;
    
    if all_layers {
        let resolver = LayerResolver::from_config(&config, &project_root);
        let layers = resolver.resolve()?;
        
        for layer in &layers {
            println!("Checking layer: {} ({})", layer.name, layer.path.display());
            check_layer(&layer.path)?;
        }
        
        // 检测跨层冲突
        check_cross_layer_conflicts(&layers)?;
    } else {
        // 现有行为：只检查项目层
        let source = project_root.join(".promptpack");
        check_layer(&source)?;
    }
    
    Ok(())
}

fn check_cross_layer_conflicts(layers: &[Layer]) -> Result<()> {
    // 收集所有 asset IDs
    let mut id_to_layers: HashMap<String, Vec<&str>> = HashMap::new();
    
    for layer in layers {
        for asset in &layer.assets {
            id_to_layers
                .entry(asset.id().to_string())
                .or_default()
                .push(&layer.name);
        }
    }
    
    // 报告覆盖
    for (id, layer_names) in &id_to_layers {
        if layer_names.len() > 1 {
            println!(
                "ℹ Asset '{}' defined in multiple layers: {}",
                id,
                layer_names.join(", ")
            );
            println!("  → Will use: {} (highest priority)", layer_names.last().unwrap());
        }
    }
    
    Ok(())
}
```

### Task 4.4: Implement `calvin migrate` Command

**File**: `src/commands/migrate.rs`

```rust
pub fn run_migrate(dry_run: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let old_lockfile = project_root.join(".promptpack/.calvin.lock");
    let new_lockfile = project_root.join("calvin.lock");
    
    let mut changes = Vec::new();
    
    // 检查 lockfile 迁移
    if old_lockfile.exists() && !new_lockfile.exists() {
        changes.push(MigrationChange::MoveLockfile {
            from: old_lockfile.clone(),
            to: new_lockfile.clone(),
        });
    }
    
    if changes.is_empty() {
        println!("Nothing to migrate.");
        return Ok(());
    }
    
    println!("Migration plan:");
    for change in &changes {
        match change {
            MigrationChange::MoveLockfile { from, to } => {
                println!("  • Move lockfile: {} → {}", from.display(), to.display());
            }
        }
    }
    
    if dry_run {
        println!("\n(Dry run - no changes made)");
        return Ok(());
    }
    
    // 执行迁移
    for change in changes {
        match change {
            MigrationChange::MoveLockfile { from, to } => {
                let content = fs::read_to_string(&from)?;
                fs::write(&to, content)?;
                fs::remove_file(&from)?;
                println!("✓ Moved lockfile");
            }
        }
    }
    
    println!("\n✓ Migration complete");
    
    Ok(())
}

enum MigrationChange {
    MoveLockfile { from: PathBuf, to: PathBuf },
}
```

### Task 4.5: Update Documentation

**Files to update**:
- `docs/configuration.md` - 添加 `[sources]` section
- `docs/command-reference.md` - 添加新命令
- `CHANGELOG.md` - 添加变更记录

**Configuration docs update**:
```markdown
## Multi-Layer Sources

Calvin supports loading prompts from multiple sources:

### Configuration

```toml
[sources]
# Enable user layer (default: true)
use_user_layer = true

# User layer path (default: ~/.calvin/.promptpack)
user_layer_path = "~/.calvin/.promptpack"

# Additional layers (optional)
additional_layers = [
  "~/team-prompts/.promptpack",
]
```

### CLI Flags

```bash
calvin deploy --source PATH      # Override project source
calvin deploy --layer PATH       # Add extra layer
calvin deploy --no-user-layer    # Disable user layer
```

### Commands

```bash
calvin layers       # Show layer stack
calvin provenance   # Show output sources
calvin projects     # List managed projects
calvin init --user  # Create user layer
```
```

## Verification

1. 运行所有新命令的测试
2. 手动验证每个命令的输出格式
3. 确认文档完整性

## Outputs

- `calvin layers` 命令
- `calvin provenance` 命令
- `calvin check --all-layers` 功能
- `calvin migrate` 命令
- 更新的文档

