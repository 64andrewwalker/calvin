# Phase 2: Global Registry

> **Priority**: Medium  
> **Estimated Effort**: 2-3 days  
> **Dependencies**: Phase 1 complete
> **Architecture Reference**: `docs/architecture/layers.md`
> **UI Reference**: `docs/ui-components-spec.md`

## Objective

实现全局 registry 追踪所有 Calvin 管理的项目，支持批量操作。

## Architecture Compliance

按四层架构分布新组件：

```
Layer 2 (Domain):
├── domain/entities/registry.rs       # Registry, ProjectEntry (~100 lines)
└── domain/ports/registry_repository.rs  # RegistryRepository trait (~30 lines)

Layer 3 (Infrastructure):
└── infrastructure/repositories/registry.rs  # TomlRegistryRepository (~150 lines)

Layer 1 (Application):
└── application/registry/use_case.rs  # RegistryUseCase (~100 lines)

Layer 0 (Presentation):
├── commands/projects.rs              # CLI handler (~80 lines)
└── ui/views/projects.rs              # View renderer (~100 lines)
```

**关键约束**:
- `Registry` entity 不依赖任何外部模块
- `TomlRegistryRepository` 实现 `RegistryRepository` port
- `RegistryUseCase` 通过 port 依赖，不直接依赖 Infrastructure
- 每个文件 < 400 行

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

### Task 2.2: Define Port and Implement Repository

**Port Definition (Layer 2 Domain)**:

**File**: `src/domain/ports/registry_repository.rs`

```rust
use crate::domain::entities::registry::{Registry, ProjectEntry};
use std::path::Path;

/// Port: Registry 持久化
pub trait RegistryRepository: Send + Sync {
    /// 加载 registry
    fn load(&self) -> Registry;
    
    /// 保存 registry
    fn save(&self, registry: &Registry) -> Result<(), RegistryError>;
    
    /// 原子更新单个项目
    fn update_project(&self, entry: ProjectEntry) -> Result<(), RegistryError>;
}

#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    #[error("Failed to access registry: {message}")]
    AccessError { message: String },
    
    #[error("Failed to serialize registry: {message}")]
    SerializationError { message: String },
}
```

**Infrastructure Implementation (Layer 3)**:

**File**: `src/infrastructure/repositories/registry.rs`

```rust
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use fs2::FileExt; // 文件锁
use crate::domain::ports::registry_repository::{RegistryRepository, RegistryError};

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

### Task 2.3: Create RegistryUseCase (Application Layer)

**File**: `src/application/registry/use_case.rs`

```rust
use crate::domain::entities::registry::{Registry, ProjectEntry};
use crate::domain::ports::registry_repository::RegistryRepository;
use std::path::Path;
use std::sync::Arc;

/// Application 层：编排 Registry 操作
pub struct RegistryUseCase {
    repository: Arc<dyn RegistryRepository>,
}

impl RegistryUseCase {
    pub fn new(repository: Arc<dyn RegistryRepository>) -> Self {
        Self { repository }
    }
    
    /// 注册/更新项目
    pub fn register_project(
        &self,
        project_path: &Path,
        lockfile_path: &Path,
        asset_count: usize,
    ) -> Result<(), RegistryError> {
        let entry = ProjectEntry {
            path: project_path.to_path_buf(),
            lockfile: lockfile_path.to_path_buf(),
            last_deployed: Utc::now(),
            asset_count,
        };
        
        self.repository.update_project(entry)
    }
    
    /// 列出所有项目
    pub fn list_projects(&self) -> Vec<ProjectEntry> {
        self.repository.load().all().to_vec()
    }
    
    /// 清理失效项目
    pub fn prune(&self) -> Result<Vec<PathBuf>, RegistryError> {
        let mut registry = self.repository.load();
        let removed = registry.prune();
        self.repository.save(&registry)?;
        Ok(removed)
    }
}
```

### Task 2.4: Auto-register on Deploy

**File**: `src/application/deploy/use_case.rs`

修改 `DeployUseCase` 依赖注入 `RegistryUseCase`：

```rust
impl DeployUseCase {
    pub fn new(
        // 现有依赖...
        registry_use_case: Option<Arc<RegistryUseCase>>,  // 可选，向后兼容
    ) -> Self {
        // ...
    }
    
    fn register_project(&self, project_path: &Path, lockfile_path: &Path, asset_count: usize) {
        if let Some(registry) = &self.registry_use_case {
            if let Err(e) = registry.register_project(project_path, lockfile_path, asset_count) {
                // 警告但不失败 - deploy 成功比 registry 更重要
                log::warn!("Failed to update registry: {}", e);
            }
        }
    }
}
```

**工厂更新** (`presentation/factory.rs`):

```rust
pub fn create_deploy_use_case(config: &Config) -> DeployUseCase {
    let registry_repo = Arc::new(TomlRegistryRepository::new());
    let registry_use_case = Arc::new(RegistryUseCase::new(registry_repo));
    
    DeployUseCase::new(
        // 现有参数...
        Some(registry_use_case),
    )
}
```

### Task 2.6: Implement `calvin projects` Command

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

**UI 设计** (遵循 `docs/ui-components-spec.md`):

**文件**: `src/ui/views/projects.rs`

```rust
use crate::ui::blocks::CommandHeader;
use crate::ui::widgets::Box;
use crate::ui::theme::{icons, dim, info, warning};

pub struct ProjectsView<'a> {
    registry: &'a Registry,
    pruned: Option<&'a [PathBuf]>,
}

impl<'a> ProjectsView<'a> {
    pub fn render(&self) {
        // 1. Header (使用 CommandHeader 组件)
        let mut header = CommandHeader::new(icons::DEPLOY, "Calvin Projects");
        header.add("Registry", "~/.calvin/registry.toml");
        header.render();
        
        println!();
        
        // 2. 显示已清理的项目 (如果有)
        if let Some(pruned) = self.pruned {
            if !pruned.is_empty() {
                for path in pruned {
                    println!("{} Removed: {}", warning(icons::WARNING), dim(path.display()));
                }
                println!();
            }
        }
        
        // 3. 项目列表 (使用 Box 组件)
        if self.registry.projects.is_empty() {
            println!("{} No projects found.", dim(icons::PENDING));
            println!();
            println!("{}", dim("Run `calvin deploy` in a project to register it."));
            return;
        }
        
        let mut project_box = Box::with_title("Managed Projects");
        project_box.with_style(BoxStyle::Info);
        
        // 表头 (使用 DIM 颜色)
        project_box.add_line(format!(
            "{:<40} {:>8} {:>16}",
            "Project", "Assets", "Last Deployed"
        ));
        project_box.add_line(dim("─".repeat(66)));
        
        // 项目列表
        for project in &self.registry.projects {
            let ago = humanize_duration(Utc::now() - project.last_deployed);
            let status_icon = if project.lockfile.exists() {
                icons::SUCCESS
            } else {
                icons::WARNING
            };
            project_box.add_line(format!(
                "{} {:<38} {:>8} {:>16}",
                status_icon,
                truncate(&project.path.display().to_string(), 38),
                project.asset_count,
                ago
            ));
        }
        
        project_box.render();
        
        // 4. 汇总
        println!();
        println!(
            "{} Total: {} projects",
            icons::SUCCESS.green(),
            self.registry.projects.len()
        );
    }
}
```

### Task 2.7: Implement `calvin clean --all`

**Files**:
- `src/commands/clean.rs` - 添加 `--all` 参数处理
- `src/ui/views/clean.rs` - 复用现有 CleanView，扩展支持批量

**UI 设计** (遵循 `docs/ui-components-spec.md`):

```rust
// src/commands/clean.rs
pub fn run_clean_all(dry_run: bool, yes: bool) -> Result<()> {
    let registry = RegistryRepository::load();
    
    if registry.projects.is_empty() {
        println!("{} No projects in registry.", dim(icons::PENDING));
        return Ok(());
    }
    
    // 1. Header
    let mut header = CommandHeader::new(icons::CLEAN, "Calvin Clean All");
    header.add("Projects", registry.projects.len().to_string());
    header.render();
    
    println!();
    
    // 2. 项目列表预览
    println!("Will clean {} projects:", registry.projects.len());
    for project in &registry.projects {
        println!("  {} {}", icons::PENDING, project.path.display());
    }
    println!();
    
    // 3. 确认 (使用 dialoguer 保持一致性)
    if !yes {
        let confirm = dialoguer::Confirm::new()
            .with_prompt("Clean all projects?")
            .default(false)
            .interact()?;
        
        if !confirm {
            println!("{} Aborted.", warning(icons::WARNING));
            return Ok(());
        }
    }
    
    // 4. 执行清理 (使用 StatusList 显示进度)
    let mut status_list = StatusList::new();
    for project in &registry.projects {
        status_list.add(project.path.display().to_string());
    }
    
    let mut total_files = 0;
    let mut errors = Vec::new();
    
    for (i, project) in registry.projects.iter().enumerate() {
        status_list.update(i, ItemStatus::InProgress);
        status_list.render();
        
        let options = CleanOptions::new().with_scope(None);
        
        if dry_run {
            // Dry run - 只统计
            match clean_use_case.preview(&project.lockfile, &options) {
                Ok(result) => {
                    total_files += result.files.len();
                    status_list.update(i, ItemStatus::Success);
                    status_list.update_detail(i, format!("{} files", result.files.len()));
                }
                Err(e) => {
                    errors.push((project.path.clone(), e.to_string()));
                    status_list.update(i, ItemStatus::Error);
                }
            }
        } else {
            // 实际清理
            match clean_use_case.execute_confirmed(&project.lockfile, &options) {
                Ok(result) => {
                    total_files += result.files_removed;
                    status_list.update(i, ItemStatus::Success);
                    status_list.update_detail(i, format!("{} removed", result.files_removed));
                }
                Err(e) => {
                    errors.push((project.path.clone(), e.to_string()));
                    status_list.update(i, ItemStatus::Error);
                }
            }
        }
    }
    
    status_list.render();
    
    // 5. 结果摘要 (使用 ResultSummary)
    println!();
    if dry_run {
        let mut summary = ResultSummary::partial("Dry Run Complete");
        summary.add_stat("projects", registry.projects.len());
        summary.add_stat("files to remove", total_files);
        if !errors.is_empty() {
            summary.add_warning(format!("{} errors", errors.len()));
        }
        summary.with_next_step("Run without --dry-run to apply changes");
        summary.render();
    } else {
        let mut summary = if errors.is_empty() {
            ResultSummary::success("Clean Complete")
        } else {
            ResultSummary::partial("Clean Completed with Errors")
        };
        summary.add_stat("projects cleaned", registry.projects.len() - errors.len());
        summary.add_stat("files removed", total_files);
        if !errors.is_empty() {
            summary.add_warning(format!("{} errors", errors.len()));
        }
        summary.render();
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

