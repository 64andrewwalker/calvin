# Scope Policy Refactoring Plan

## 问题现状

### 1. 分散的 Scope 处理逻辑
- `deploy` 命令在 `runner.rs::scan_assets()` 中应用 `ScopePolicy`
- `watch` 命令在 `watcher.rs::perform_sync_incremental()` 中手动设置 scope
- `diff` 命令不支持 `--home`，只做项目级比较
- 交互式命令需要跟随用户配置，可能同时包含项目级和用户级

### 2. 部署目标类型
| 场景 | 项目级 Assets | 用户级 Assets | 目标目录 |
|-----|--------------|--------------|---------|
| `deploy` (项目) | ✅ 编译到项目目录 | ✅ 编译到用户目录 | 项目根目录 + ~ |
| `deploy --home` | ❌ 强制为用户级 | ✅ | ~ |
| `watch` (项目) | ✅ | ✅ | 项目根目录 + ~ |
| `watch --home` | ❌ 强制为用户级 | ✅ | ~ |
| `diff` (当前) | ✅ | ❌ 不支持 | 项目根目录 |
| `diff --home` (待实现) | ❌ | ✅ | ~ |

### 3. Lockfile 问题
- 当前 lockfile 混合了项目级和用户级条目
- 不同部署目标应该有独立的状态追踪

## 设计方案

### Phase 1: 统一 Scope 处理抽象

#### 1.1 重构 `ScopePolicy` 为核心类型

```rust
// src/sync/scope.rs (新文件)

/// 部署目标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentTarget {
    /// 项目目录
    Project,
    /// 用户主目录
    Home,
    /// 两者都部署（按 asset 自身的 scope）
    Both,
}

/// Scope 处理策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopePolicy {
    /// 保留 asset 原有 scope
    Keep,
    /// 只保留 scope: project 的 assets
    ProjectOnly,
    /// 只保留 scope: user 的 assets
    UserOnly,
    /// 强制所有 assets 变为 scope: user
    ForceUser,
    /// 强制所有 assets 变为 scope: project
    ForceProject,
}

impl ScopePolicy {
    /// 根据部署目标确定 scope policy
    pub fn from_target(target: DeploymentTarget) -> Self {
        match target {
            DeploymentTarget::Project => ScopePolicy::Keep,  // 保留原有 scope
            DeploymentTarget::Home => ScopePolicy::ForceUser,
            DeploymentTarget::Both => ScopePolicy::Keep,
        }
    }
    
    /// 应用 policy 到 assets
    pub fn apply(&self, assets: Vec<PromptAsset>) -> Vec<PromptAsset> {
        match self {
            ScopePolicy::Keep => assets,
            ScopePolicy::ProjectOnly => assets.into_iter()
                .filter(|a| a.frontmatter.scope == Scope::Project)
                .collect(),
            ScopePolicy::UserOnly => assets.into_iter()
                .filter(|a| a.frontmatter.scope == Scope::User)
                .collect(),
            ScopePolicy::ForceUser => assets.into_iter()
                .map(|mut a| { a.frontmatter.scope = Scope::User; a })
                .collect(),
            ScopePolicy::ForceProject => assets.into_iter()
                .map(|mut a| { a.frontmatter.scope = Scope::Project; a })
                .collect(),
        }
    }
}
```

#### 1.2 创建统一的 Asset Pipeline

```rust
// src/sync/pipeline.rs (新文件)

/// 统一的 asset 处理管道
pub struct AssetPipeline {
    source: PathBuf,
    config: Config,
    scope_policy: ScopePolicy,
    targets: Vec<Target>,
}

impl AssetPipeline {
    pub fn new(source: PathBuf, config: Config) -> Self {
        Self {
            source,
            config,
            scope_policy: ScopePolicy::Keep,
            targets: vec![],
        }
    }
    
    pub fn with_scope_policy(mut self, policy: ScopePolicy) -> Self {
        self.scope_policy = policy;
        self
    }
    
    pub fn with_targets(mut self, targets: Vec<Target>) -> Self {
        self.targets = targets;
        self
    }
    
    /// Parse + Apply Scope Policy + Compile
    pub fn compile(&self) -> CalvinResult<Vec<OutputFile>> {
        let assets = parse_directory(&self.source)?;
        let filtered = self.scope_policy.apply(assets);
        compile_assets(&filtered, &self.targets, &self.config)
    }
    
    /// Incremental version for watch mode
    pub fn compile_incremental(
        &self,
        changed_files: &[PathBuf],
        cache: &mut IncrementalCache,
    ) -> CalvinResult<Vec<OutputFile>> {
        let mut assets = parse_incremental(&self.source, changed_files, cache)?;
        let filtered = self.scope_policy.apply(assets);
        compile_assets(&filtered, &self.targets, &self.config)
    }
}
```

### Phase 2: 分离 Lockfile 策略

#### 2.1 Lockfile 键策略

```rust
// src/sync/lockfile.rs 扩展

/// Lockfile 存储策略
pub enum LockfileStrategy {
    /// 单一 lockfile（当前行为）
    Single,
    /// 按目标分离
    PerTarget {
        project: PathBuf,  // .promptpack/.calvin.lock
        home: PathBuf,     // .promptpack/.calvin.home.lock
    },
}
```

**决定**：暂时保持单一 lockfile，但确保：
- 项目级路径（如 `.agent/workflows/`）只在项目模式下记录
- 用户级路径（如 `~/.claude/`）可能在两种模式下都出现

#### 2.2 路径规范化

所有 lockfile 条目使用**完整路径**或**带前缀的相对路径**：
- 项目级：`project:.agent/workflows/xxx.md`
- 用户级：`home:~/.claude/commands/xxx.md`

### Phase 3: 重构各命令

#### 3.1 `deploy` 命令

```rust
// 已经正确使用 ScopePolicy，只需迁移到新的 AssetPipeline
fn compile_outputs(&self, assets: &[PromptAsset]) -> Result<Vec<OutputFile>> {
    let pipeline = AssetPipeline::new(self.source.clone(), self.config.clone())
        .with_scope_policy(self.scope_policy)
        .with_targets(self.get_targets());
    
    pipeline.compile()
}
```

#### 3.2 `watch` 命令

```rust
fn perform_sync_incremental(...) -> CalvinResult<SyncResult> {
    let scope_policy = if options.deploy_to_home {
        ScopePolicy::ForceUser
    } else {
        ScopePolicy::Keep
    };
    
    let pipeline = AssetPipeline::new(options.source.clone(), options.config.clone())
        .with_scope_policy(scope_policy)
        .with_targets(options.targets.clone());
    
    let outputs = pipeline.compile_incremental(changed_files, cache)?;
    // ... rest of sync logic
}
```

#### 3.3 `diff` 命令（新增 --home 支持）

```rust
pub fn cmd_diff(
    source: &Path, 
    home: bool,  // 新增参数
    json: bool
) -> Result<()> {
    let scope_policy = if home {
        ScopePolicy::ForceUser
    } else {
        ScopePolicy::Keep
    };
    
    let pipeline = AssetPipeline::new(source.to_path_buf(), config)
        .with_scope_policy(scope_policy);
    
    let outputs = pipeline.compile()?;
    
    // 根据 scope 确定比较目标
    let compare_root = if home {
        dirs::home_dir().unwrap_or_default()
    } else {
        project_root
    };
    
    // ... diff logic
}
```

#### 3.4 交互式命令

```rust
// interactive.rs

fn handle_deploy_selection(&self, choice: usize) -> Result<()> {
    match choice {
        1 => self.deploy(DeploymentTarget::Project),
        2 => self.deploy(DeploymentTarget::Home),
        3 => self.deploy(DeploymentTarget::Both),  // 新增：两者都部署
        _ => Err(anyhow!("Invalid choice")),
    }
}

fn deploy(&self, target: DeploymentTarget) -> Result<()> {
    let scope_policy = ScopePolicy::from_target(target);
    
    // 根据 target 决定部署到哪里
    match target {
        DeploymentTarget::Project => {
            // 只部署到项目目录
            // scope: project 的 assets -> 项目目录
            // scope: user 的 assets -> ~ (保持原有行为)
        }
        DeploymentTarget::Home => {
            // 只部署到 home
            // 所有 assets 强制为 scope: user
        }
        DeploymentTarget::Both => {
            // 按 asset 自身 scope 分别部署
            // scope: project -> 项目目录
            // scope: user -> ~
        }
    }
}
```

### Phase 4: 实现步骤

#### Step 1: 创建核心抽象 (1-2 hours)
1. 创建 `src/sync/scope.rs`
2. 实现 `ScopePolicy` 和 `DeploymentTarget`
3. 添加测试

#### Step 2: 创建 AssetPipeline (1 hour)
1. 创建 `src/sync/pipeline.rs`
2. 实现基本 pipeline 逻辑
3. 添加测试

#### Step 3: 重构 deploy 命令 (30 min)
1. 迁移到使用 `AssetPipeline`
2. 删除 `deploy/targets.rs` 中的重复定义
3. 运行测试验证

#### Step 4: 重构 watch 命令 (30 min)
1. 迁移到使用 `AssetPipeline`
2. 删除手动 scope 处理代码
3. 运行测试验证

#### Step 5: 扩展 diff 命令 (1 hour)
1. 添加 `--home` 参数
2. 集成 `AssetPipeline`
3. 调整比较逻辑以支持多目标
4. 添加测试

#### Step 6: 更新交互式命令 (1 hour)
1. 添加 "Both" 部署选项
2. 集成配置驱动的行为
3. 测试所有交互流程

#### Step 7: Lockfile 清理 (30 min)
1. 确保只记录相关目标的条目
2. 可选：添加 lockfile 清理命令

### 预期结果

1. **统一的 Scope 处理**：所有命令使用相同的 `ScopePolicy` 抽象
2. **`diff --home` 支持**：可以预览 home 目录的变化
3. **交互式灵活性**：用户可以选择部署到哪里
4. **干净的 Lockfile**：只包含相关目标的条目
5. **可维护性**：新增命令只需使用 `AssetPipeline`

### 风险和缓解

| 风险 | 缓解措施 |
|-----|---------|
| 破坏现有行为 | 每步都运行完整测试套件 |
| Lockfile 迁移 | 提供 `calvin migrate lockfile` 命令 |
| 用户混淆 | 更新帮助文本和文档 |

### 非目标（未来考虑）

- 多 lockfile 策略（暂保持单一 lockfile）
- 远程目标的 scope 处理
- GUI 界面

---

## TODO 清单

### Phase 1: 统一 Scope 处理抽象
- [ ] 创建 `src/sync/scope.rs`
- [ ] 实现 `DeploymentTarget` 枚举 (Project | Home | Both)
- [ ] 实现 `ScopePolicy` 枚举及 `apply()` 方法
- [ ] 添加 `ScopePolicy::from_target()` 便捷方法
- [ ] 添加单元测试

### Phase 2: 创建 AssetPipeline
- [ ] 创建 `src/sync/pipeline.rs`
- [ ] 实现 `AssetPipeline` struct
- [ ] 实现 `compile()` 方法
- [ ] 实现 `compile_incremental()` 方法 (for watch mode)
- [ ] 添加单元测试
- [ ] 更新 `src/sync/mod.rs` 导出

### Phase 3: 重构 deploy 命令
- [ ] 删除 `deploy/targets.rs` 中的 `ScopePolicy` 重复定义
- [ ] 更新 `deploy/runner.rs` 使用 `AssetPipeline`
- [ ] 更新 `deploy_v2/runner.rs` 使用 `AssetPipeline`
- [ ] 运行测试验证

### Phase 4: 重构 watch 命令
- [ ] 更新 `watcher.rs` 使用 `AssetPipeline`
- [ ] 删除手动 scope 处理代码
- [ ] 运行测试验证

### Phase 5: 扩展 diff 命令
- [ ] 添加 `--home` CLI 参数到 `cli.rs`
- [ ] 更新 `cmd_diff()` 支持 `home` 参数
- [ ] 集成 `AssetPipeline`
- [ ] 调整比较逻辑以支持 home 目标
- [ ] 添加测试
- [ ] 更新帮助文档

### Phase 6: 更新交互式命令
- [ ] 添加 "Both" 部署选项到菜单
- [ ] 实现 `DeploymentTarget::Both` 逻辑
- [ ] 配置驱动行为（记住用户上次选择）
- [ ] 测试所有交互流程

### Phase 7: Lockfile 清理
- [ ] 确保只记录相关目标的条目
- [ ] 验证 `watch --home` 不再添加项目级路径
- [ ] 可选：添加 `calvin lockfile clean` 命令

### 验收标准
- [ ] 所有现有测试通过
- [ ] `deploy --home` 行为不变
- [ ] `watch --home` 只记录用户级路径
- [ ] `diff --home` 可用
- [ ] 交互式可选择 Project / Home / Both
- [ ] 文档更新

