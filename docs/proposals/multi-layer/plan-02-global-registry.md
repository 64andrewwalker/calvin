# Phase 2: Global Registry

> **Priority**: Medium  
> **Estimated Effort**: 2-3 days  
> **Dependencies**: Phase 1 complete

## Objective

实现全局 registry 追踪所有 Calvin 管理的项目，支持批量操作。

## Key Concepts

### Registry

```rust
pub struct Registry {
    pub version: u32,
    pub projects: Vec<ProjectEntry>,
}

pub struct ProjectEntry {
    pub path: PathBuf,
    pub lockfile: PathBuf,
    pub last_deployed: DateTime<Utc>,
    pub asset_count: usize,
}
```

### Location

```
~/.calvin/registry.toml
```

## Detailed Tasks

### Task 2.1: Define Types

**File**: `src/domain/entities/registry.rs`

```rust
use std::path::PathBuf;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct ProjectEntry {
    pub path: PathBuf,
    pub lockfile: PathBuf,
    pub last_deployed: DateTime<Utc>,
    pub asset_count: usize,
}

#[derive(Debug, Clone, Default)]
pub struct Registry {
    pub version: u32,
    pub projects: Vec<ProjectEntry>,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            version: 1,
            projects: Vec::new(),
        }
    }
    
    /// 插入或更新项目
    pub fn upsert(&mut self, entry: ProjectEntry) {
        if let Some(existing) = self.projects.iter_mut().find(|p| p.path == entry.path) {
            *existing = entry;
        } else {
            self.projects.push(entry);
        }
    }
    
    /// 移除项目
    pub fn remove(&mut self, path: &Path) -> bool {
        let len_before = self.projects.len();
        self.projects.retain(|p| p.path != path);
        self.projects.len() != len_before
    }
    
    /// 清理不存在的项目
    pub fn prune(&mut self) -> Vec<PathBuf> {
        let (valid, invalid): (Vec<_>, Vec<_>) = self.projects
            .drain(..)
            .partition(|p| p.lockfile.exists());
        
        let removed: Vec<_> = invalid.into_iter().map(|p| p.path).collect();
        self.projects = valid;
        removed
    }
    
    /// 获取所有项目
    pub fn all(&self) -> &[ProjectEntry] {
        &self.projects
    }
}
```

**Tests**:
```rust
#[test]
fn registry_upsert_new() {
    let mut registry = Registry::new();
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/project"),
        lockfile: PathBuf::from("/project/calvin.lock"),
        last_deployed: Utc::now(),
        asset_count: 5,
    });
    assert_eq!(registry.projects.len(), 1);
}

#[test]
fn registry_upsert_existing() {
    let mut registry = Registry::new();
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/project"),
        lockfile: PathBuf::from("/project/calvin.lock"),
        last_deployed: Utc::now(),
        asset_count: 5,
    });
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/project"),
        lockfile: PathBuf::from("/project/calvin.lock"),
        last_deployed: Utc::now(),
        asset_count: 10, // 更新
    });
    assert_eq!(registry.projects.len(), 1);
    assert_eq!(registry.projects[0].asset_count, 10);
}

#[test]
fn registry_prune_removes_missing() {
    let dir = tempdir().unwrap();
    let existing = dir.path().join("exists/calvin.lock");
    fs::create_dir_all(existing.parent().unwrap()).unwrap();
    fs::write(&existing, "").unwrap();
    
    let mut registry = Registry::new();
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/exists"),
        lockfile: existing,
        last_deployed: Utc::now(),
        asset_count: 5,
    });
    registry.upsert(ProjectEntry {
        path: PathBuf::from("/missing"),
        lockfile: PathBuf::from("/missing/calvin.lock"),
        last_deployed: Utc::now(),
        asset_count: 3,
    });
    
    let removed = registry.prune();
    assert_eq!(removed.len(), 1);
    assert_eq!(removed[0], PathBuf::from("/missing"));
    assert_eq!(registry.projects.len(), 1);
}
```

### Task 2.2: Implement Repository

**File**: `src/infrastructure/repositories/registry.rs`

```rust
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use fs2::FileExt; // 文件锁

#[derive(Serialize, Deserialize)]
struct TomlProjectEntry {
    path: PathBuf,
    lockfile: PathBuf,
    last_deployed: DateTime<Utc>,
    asset_count: usize,
}

#[derive(Serialize, Deserialize)]
struct TomlRegistry {
    version: u32,
    projects: Vec<TomlProjectEntry>,
}

pub struct RegistryRepository;

impl RegistryRepository {
    /// 获取 registry 路径
    pub fn path() -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join(".calvin/registry.toml"))
            .unwrap_or_else(|| PathBuf::from("~/.calvin/registry.toml"))
    }
    
    /// 加载 registry
    pub fn load() -> Registry {
        let path = Self::path();
        if !path.exists() {
            return Registry::new();
        }
        
        match fs::read_to_string(&path) {
            Ok(content) => {
                match toml::from_str::<TomlRegistry>(&content) {
                    Ok(toml_reg) => Self::from_toml(toml_reg),
                    Err(_) => Registry::new(),
                }
            }
            Err(_) => Registry::new(),
        }
    }
    
    /// 保存 registry (带文件锁)
    pub fn save(registry: &Registry) -> Result<(), RegistryError> {
        let path = Self::path();
        
        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // 获取文件锁
        let lock_path = path.with_extension("lock");
        let lock_file = fs::File::create(&lock_path)?;
        lock_file.lock_exclusive()?;
        
        // 写入
        let toml_reg = Self::to_toml(registry);
        let content = toml::to_string_pretty(&toml_reg)?;
        fs::write(&path, content)?;
        
        // 释放锁
        lock_file.unlock()?;
        
        Ok(())
    }
    
    /// 更新单个项目 (原子操作)
    pub fn update_project(entry: ProjectEntry) -> Result<(), RegistryError> {
        let path = Self::path();
        
        // 获取文件锁
        let lock_path = path.with_extension("lock");
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let lock_file = fs::File::create(&lock_path)?;
        lock_file.lock_exclusive()?;
        
        // 读取、更新、写入
        let mut registry = Self::load();
        registry.upsert(entry);
        
        let toml_reg = Self::to_toml(&registry);
        let content = toml::to_string_pretty(&toml_reg)?;
        fs::write(&path, content)?;
        
        lock_file.unlock()?;
        
        Ok(())
    }
    
    fn from_toml(toml: TomlRegistry) -> Registry { ... }
    fn to_toml(registry: &Registry) -> TomlRegistry { ... }
}
```

### Task 2.3: Auto-register on Deploy

**File**: `src/application/deploy/use_case.rs`

```rust
impl DeployUseCase {
    fn register_project(&self, project_path: &Path, lockfile_path: &Path, asset_count: usize) {
        let entry = ProjectEntry {
            path: project_path.to_path_buf(),
            lockfile: lockfile_path.to_path_buf(),
            last_deployed: Utc::now(),
            asset_count,
        };
        
        if let Err(e) = RegistryRepository::update_project(entry) {
            // 警告但不失败
            eprintln!("⚠ Failed to update registry: {}", e);
        }
    }
}
```

### Task 2.4: Implement `calvin projects` Command

**File**: `src/commands/projects.rs`

```rust
pub fn run_projects(prune: bool, json: bool) -> Result<()> {
    let mut registry = RegistryRepository::load();
    
    if prune {
        let removed = registry.prune();
        if !removed.is_empty() {
            RegistryRepository::save(&registry)?;
            for path in &removed {
                eprintln!("Removed: {}", path.display());
            }
        }
    }
    
    if json {
        let output = serde_json::to_string_pretty(&registry.all())?;
        println!("{}", output);
    } else {
        render_projects_table(&registry);
    }
    
    Ok(())
}
```

**UI**:
```rust
fn render_projects_table(registry: &Registry) {
    println!("╭─────────────────────────────────────────────────────────────────╮");
    println!("│  📂 Calvin-managed Projects                                     │");
    println!("╰─────────────────────────────────────────────────────────────────╯");
    println!();
    
    if registry.projects.is_empty() {
        println!("No projects found. Run `calvin deploy` in a project to register it.");
        return;
    }
    
    println!("┌──────────────────────────────────┬─────────┬──────────────────┐");
    println!("│ Project                          │ Assets  │ Last Deployed    │");
    println!("├──────────────────────────────────┼─────────┼──────────────────┤");
    
    for project in &registry.projects {
        let ago = humanize_duration(Utc::now() - project.last_deployed);
        println!(
            "│ {:<32} │ {:>7} │ {:<16} │",
            truncate(&project.path.display().to_string(), 32),
            project.asset_count,
            ago
        );
    }
    
    println!("└──────────────────────────────────┴─────────┴──────────────────┘");
    println!();
    println!("Total: {} projects", registry.projects.len());
}
```

### Task 2.5: Implement `calvin clean --all`

**File**: `src/commands/clean.rs`

```rust
pub fn run_clean_all(dry_run: bool, yes: bool) -> Result<()> {
    let registry = RegistryRepository::load();
    
    if registry.projects.is_empty() {
        eprintln!("No projects in registry.");
        return Ok(());
    }
    
    println!("Found {} projects:", registry.projects.len());
    for project in &registry.projects {
        println!("  - {}", project.path.display());
    }
    
    if !yes {
        // 确认
        print!("Clean all projects? [y/N] ");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Aborted.");
            return Ok(());
        }
    }
    
    for project in &registry.projects {
        println!("\nCleaning {}...", project.path.display());
        let options = CleanOptions::new().with_scope(None);
        
        if dry_run {
            let result = clean_use_case.execute(&project.lockfile, &options);
            // 显示预览
        } else {
            let result = clean_use_case.execute_confirmed(&project.lockfile, &options);
            // 显示结果
        }
    }
    
    Ok(())
}
```

## Verification

1. 运行 `cargo test registry`
2. 手动测试：
   - 在项目 A 运行 `calvin deploy`
   - 在项目 B 运行 `calvin deploy`
   - 运行 `calvin projects`
   - 验证两个项目都列出

## Outputs

- `Registry` 和 `ProjectEntry` 类型
- `RegistryRepository`
- `calvin projects` 命令
- `calvin clean --all` 功能
- 自动注册逻辑

