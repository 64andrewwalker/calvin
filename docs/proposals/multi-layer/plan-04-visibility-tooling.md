# Phase 4: Visibility & Tooling

> **Priority**: Low (Polish)  
> **Estimated Effort**: 1-2 days  
> **Dependencies**: Phase 1-3 complete
> **UI Reference**: `docs/ui-components-spec.md`

## Objective

添加可视化命令和工具，帮助用户理解和调试多层系统。

## UI Design Principles

所有新命令必须遵循 `docs/ui-components-spec.md` 规范：

1. **使用设计令牌** (`src/ui/theme.rs`)
   - 颜色：SUCCESS (green), ERROR (red), WARNING (yellow), INFO (cyan), DIM (dimmed)
   - 图标：统一使用 `icons` 模块
   - 边框：使用圆角边框字符

2. **使用现有组件**
   - `CommandHeader` - 命令头部
   - `Box` - 边框容器
   - `ResultSummary` - 结果摘要
   - `StatusList` - 状态列表

3. **降级支持**
   - 无 Unicode 时降级到 ASCII
   - 无颜色时降级到纯文本
   - CI 环境禁用动画

## Detailed Tasks

### Task 4.1: Implement `calvin layers` Command

**Files**:
- `src/commands/layers.rs` - Command handler
- `src/ui/views/layers.rs` - View renderer

**UI 规范遵循**:
- 使用 `CommandHeader` 显示命令头部
- 使用 `Box` 组件包裹层列表
- 使用 `StatusList` 显示每层详情
- 使用 `ResultSummary` 显示合并统计

```rust
// src/commands/layers.rs
pub fn run_layers(json: bool) -> Result<()> {
    let config = Config::load_or_default(Some(&std::env::current_dir()?));
    let project_root = std::env::current_dir()?;
    
    let resolver = LayerResolver::from_config(&config, &project_root);
    let layers = resolver.resolve()?;
    
    if json {
        let output = serde_json::to_string_pretty(&layers)?;
        println!("{}", output);
    } else {
        let view = LayersView::new(&layers);
        view.render();
    }
    
    Ok(())
}
```

```rust
// src/ui/views/layers.rs
use crate::ui::blocks::{CommandHeader, ResultSummary};
use crate::ui::widgets::StatusList;
use crate::ui::theme::icons;

pub struct LayersView<'a> {
    layers: &'a [Layer],
}

impl<'a> LayersView<'a> {
    pub fn new(layers: &'a [Layer]) -> Self {
        Self { layers }
    }
    
    pub fn render(&self) {
        // 1. Header
        let mut header = CommandHeader::new(icons::CHECK, "Calvin Layers");
        header.add("Project", std::env::current_dir().unwrap().display().to_string());
        header.render();
        
        // 2. Layer stack (使用 Box 组件)
        println!();
        let mut layer_box = Box::with_title("Layer Stack (high → low)");
        layer_box.with_style(BoxStyle::Info);
        
        for (i, layer) in self.layers.iter().enumerate().rev() {
            let priority = self.layers.len() - i;
            let layer_type = match layer.layer_type {
                LayerType::User => format!("{} [user]", icons::PENDING),
                LayerType::Custom => format!("{} [custom]", icons::PARTIAL),
                LayerType::Project => format!("{} [project]", icons::SELECTED),
            };
            layer_box.add_line(format!(
                "{}. {} {} ({} assets)",
                priority,
                layer_type,
                truncate(&layer.path.display().to_string(), 35),
                layer.assets.len()
            ));
        }
        layer_box.render();
        
        // 3. Summary
        let total_assets: usize = self.layers.iter().map(|l| l.assets.len()).sum();
        let merged = merge_layers(self.layers);
        let overridden = total_assets - merged.len();
        
        println!();
        println!(
            "{} Merged: {} assets ({} overridden by higher layers)",
            icons::SUCCESS.green(),
            merged.len(),
            overridden
        );
    }
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

**Files**:
- `src/commands/provenance.rs` - Command handler
- `src/ui/views/provenance.rs` - View renderer

**UI 规范遵循**:
- 使用 `CommandHeader` 显示命令头部
- 使用 `StatusList` 显示每个输出文件的来源
- 使用 DIM 颜色显示次要信息
- 使用 `icons::ARROW` 显示来源指向

```rust
// src/commands/provenance.rs
pub fn run_provenance(json: bool, filter: Option<String>) -> Result<()> {
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
        let view = ProvenanceView::new(&lockfile, filter.as_deref());
        view.render();
    }
    
    Ok(())
}
```

```rust
// src/ui/views/provenance.rs
use crate::ui::blocks::CommandHeader;
use crate::ui::theme::{icons, colors, dim, info};

pub struct ProvenanceView<'a> {
    lockfile: &'a Lockfile,
    filter: Option<&'a str>,
}

impl<'a> ProvenanceView<'a> {
    pub fn render(&self) {
        // 1. Header
        let mut header = CommandHeader::new(icons::CHECK, "Calvin Provenance");
        header.add("Lockfile", "calvin.lock");
        header.render();
        
        println!();
        
        let entries: Vec<_> = self.lockfile.entries()
            .filter(|(k, _)| self.filter.map_or(true, |f| k.contains(f)))
            .collect();
        
        if entries.is_empty() {
            println!("{} No outputs found.", dim("○"));
            return;
        }
        
        // 2. Output list with provenance
        for (key, entry) in &entries {
            let (_, path) = Lockfile::parse_key(key).unwrap_or((Scope::Project, key));
            
            // 输出路径 (INFO 颜色)
            println!("{}", info(path));
            
            // 来源信息 (DIM 颜色，使用 ARROW 图标)
            if let Some(source_file) = entry.source_file() {
                println!("  {} Source: {}", icons::ARROW, dim(source_file.display()));
            }
            if let Some(layer) = entry.source_layer() {
                let layer_icon = match layer.as_str() {
                    "project" => icons::SELECTED,
                    "user" => icons::PENDING,
                    _ => icons::PARTIAL,
                };
                println!("  {} Layer:  {} {}", icons::ARROW, layer_icon, layer);
            }
            if let Some(overrides) = entry.overrides() {
                println!(
                    "  {} {}",
                    icons::WARNING.yellow(),
                    format!("Overrides asset from '{}' layer", overrides).yellow()
                );
            }
            println!();
        }
        
        // 3. Summary
        let layers: HashSet<_> = entries.iter()
            .filter_map(|(_, e)| e.source_layer())
            .collect();
        
        println!("{}", dim(format!(
            "Total: {} output files from {} layers",
            entries.len(),
            layers.len()
        )));
    }
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

