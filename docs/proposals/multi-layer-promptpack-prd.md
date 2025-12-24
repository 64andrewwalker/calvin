# Multi-Layer PromptPack Sources PRD

> **Status**: Draft  
> **Author**: [Maintainer]  
> **Created**: 2024-12-24  
> **Target Version**: 0.4.0

## 1. Problem Statement

### Current Limitation

Calvin 目前只支持单一来源的 `.promptpack/`：执行 `calvin deploy` 时，只会读取当前工作目录下的 `.promptpack/` 目录。

### Pain Points

1. **违背唯一来源原则 (Single Source of Truth)**
   - 用户如果想在多个项目中复用同一套 prompts，必须将 `.promptpack/` 复制到每个项目
   - 这导致 prompt 维护分散，违背 DRY 原则

2. **无法分层管理**
   - 缺少"个人偏好"层：用户无法维护跨项目的个人 prompt 规范
   - 缺少"团队共享"层：团队无法维护一套通用 prompts，同时允许项目覆盖

3. **配置不灵活**
   - 无法指定外部 promptpack 路径
   - 无法从多个来源合并 assets

---

## 2. Goals

### Primary Goals

1. 实现多层级 promptpack 来源，支持合并与覆盖语义
2. 支持 `~/.calvin/.promptpack` 作为用户级全局 promptpack
3. 允许用户通过配置指定额外的 promptpack 路径

### Non-Goals

- 不实现远程 promptpack 源（如 Git URL、HTTP）—— 留给未来版本
- 不实现 promptpack 的版本管理或锁定机制
- 不改变现有单一 `.promptpack/` 的行为（向后兼容）

---

## 3. Design Philosophy

### Why This Doesn't Violate Single Source of Truth

表面上看，多个 layer 来源似乎违反了单一来源原则。实际上恰恰相反：

**当前的问题（违反单一来源）**：

```
项目A/.promptpack/code-style.md    ← 复制
项目B/.promptpack/code-style.md    ← 复制  
项目C/.promptpack/code-style.md    ← 复制
```

同一份规则被复制 3 次。修改时必须改 3 个地方。

**新设计的解决方案（实现单一来源）**：

```
~/.calvin/.promptpack/code-style.md   ← 唯一定义（一次）
                      ↓ 被引用
项目A/.promptpack/    （可能有项目特有的覆盖）
项目B/.promptpack/    （可能有项目特有的覆盖）
项目C/.promptpack/    （可能有项目特有的覆盖）
```

**核心区别**：
- 每个信息只在**一个地方定义**
- 项目层只用于**覆盖/扩展**，不需要重复定义通用内容
- 修改全局规则时，只需改一个文件

### Industry Precedent

这种分层设计是软件工程的常见最佳实践：

| Tool | Global Layer | Project Layer | Principle |
|------|--------------|---------------|-----------|
| Git | `~/.gitconfig` | `.git/config` | 项目覆盖全局 |
| npm | `~/.npmrc` | `.npmrc` | 项目覆盖全局 |
| ESLint | 父目录 `.eslintrc` | 项目 `.eslintrc` | 就近覆盖 |
| CSS | 父选择器 | 子选择器 | 级联覆盖 |
| Cargo | `~/.cargo/config.toml` | `.cargo/config.toml` | 项目覆盖全局 |

---

## 4. Design Overview

### 4.1 Promptpack Layer Hierarchy

优先级从低到高（后者覆盖前者）：

```
┌─────────────────────────────────────────────────────────────────┐
│  Layer 3: Project Layer (highest priority)                     │
│  Path: ./.promptpack/                                           │
│  Scope: 当前项目特有的 prompts，覆盖所有低层级                    │
└─────────────────────────────────────────────────────────────────┘
                              ▲ overrides
┌─────────────────────────────────────────────────────────────────┐
│  Layer 2: Custom Layers (configurable, multiple allowed)       │
│  Path: 用户配置的任意路径，如 ~/work/team-prompts/.promptpack    │
│  Scope: 团队共享、特定领域的 prompts                            │
└─────────────────────────────────────────────────────────────────┘
                              ▲ overrides
┌─────────────────────────────────────────────────────────────────┐
│  Layer 1: User Layer (lowest priority)                         │
│  Path: ~/.calvin/.promptpack                                    │
│  Scope: 用户个人的全局 prompts，跨所有项目生效                   │
└─────────────────────────────────────────────────────────────────┘
```

### 4.2 Merge Semantics

**Asset 合并规则：**

1. **相同 `id` 的 asset**：高层级完全覆盖低层级（不做字段级合并）
2. **不同 `id` 的 asset**：全部包含在最终编译输出中
3. **config.toml 配置**：按层级合并，高层级覆盖低层级的同名字段

**示例：**

```
~/.calvin/.promptpack/
├── policies/
│   └── code-style.md       # id: code-style
│   └── security.md         # id: security
└── actions/
    └── review.md           # id: review

./project/.promptpack/
├── policies/
│   └── code-style.md       # id: code-style (覆盖用户层)
└── actions/
    └── deploy.md           # id: deploy (新增)

最终编译输出：
- policies: code-style (来自 project), security (来自 user)
- actions: review (来自 user), deploy (来自 project)
```

### 4.3 Configuration Schema

扩展 `.promptpack/config.toml` 和 `~/.config/calvin/config.toml`：

```toml
# ~/.config/calvin/config.toml (用户配置)

[sources]
# 是否启用用户层 promptpack（默认 true）
use_user_layer = true

# 用户层 promptpack 路径（默认 ~/.calvin/.promptpack）
# 用户也可以指定其他路径，例如 dotfiles repo
user_layer_path = "~/.calvin/.promptpack"

# 自定义示例：指向 dotfiles repo
# user_layer_path = "~/dotfiles/.promptpack"

# 额外的 promptpack 路径（可配置多个，按数组顺序优先级递增）
additional_layers = [
  "~/work/team-prompts/.promptpack",
  "/shared/company-prompts/.promptpack",
]

# 是否禁用项目层（默认 false，仅用于调试或特殊场景）
disable_project_layer = false
```

**用户层路径配置说明**：
- `user_layer_path` 可以是任意路径，不必在 `~/.calvin/` 下
- 常见用法：指向 dotfiles repo 中的 promptpack，方便跨机器同步
- 支持 `~` 展开和绝对路径

```toml
# .promptpack/config.toml (项目配置)

[sources]
# 项目可以选择忽略用户层（默认 false）
ignore_user_layer = false

# 项目可以选择忽略额外层（默认 false）
ignore_additional_layers = false
```

### 4.4 CLI Interface

#### New Flags

```bash
# 显式指定 promptpack 路径（覆盖默认项目层检测）
calvin deploy --source ~/my-prompts/.promptpack

# 显式指定额外的 promptpack 层（在项目层之前应用）
calvin deploy --layer ~/team-prompts/.promptpack

# 可指定多个（优先级按顺序递增）
calvin deploy --layer ~/base-prompts --layer ~/team-prompts

# 忽略用户层，仅使用项目层
calvin deploy --no-user-layer

# 忽略所有额外层
calvin deploy --no-additional-layers

# 只使用指定路径，忽略所有层级检测
calvin deploy --source ~/custom-prompts --no-user-layer --no-additional-layers

# 显示层级解析信息
calvin deploy -v  # verbose 模式下显示 layer 来源
```

**`--source` vs `--layer` 的区别**：
- `--source PATH`: 替换项目层的检测路径（默认 `./.promptpack`）
- `--layer PATH`: 添加额外的层（在其他层之后、项目层之前应用）

#### New Commands

```bash
# 查看当前解析的 layer 层级
calvin layers

# 输出示例：
# Layer Stack (highest priority first):
#   3. [project] ./.promptpack/ (12 assets)
#   2. [custom]  ~/team-prompts/.promptpack (5 assets)
#   1. [user]    ~/.calvin/.promptpack (8 assets)
#
# Merged assets: 18 (7 overridden)

# 初始化用户层 promptpack
calvin init --user
# Creates: ~/.calvin/.promptpack/
#          ~/.calvin/.promptpack/config.toml
#          ~/.calvin/.promptpack/policies/.gitkeep
#          ~/.calvin/.promptpack/actions/.gitkeep
```

---

## 5. Detailed Design

### 5.1 Layer Resolution Algorithm

```rust
/// Resolve all promptpack layers in priority order (lowest to highest)
fn resolve_layers(config: &Config, cli_args: &CliArgs) -> Vec<PromptpackLayer> {
    let mut layers = Vec::new();
    
    // Layer 1: User layer (lowest priority)
    if config.sources.use_user_layer && !cli_args.no_user_layer {
        if let Some(user_layer) = load_user_layer(&config.sources.user_layer_path) {
            layers.push(user_layer);
        }
    }
    
    // Layer 2: Additional configured layers
    if !cli_args.no_additional_layers {
        for path in &config.sources.additional_layers {
            if let Some(layer) = load_layer(path) {
                layers.push(layer);
            }
        }
    }
    
    // Layer 2.5: CLI-specified layers
    for path in &cli_args.layers {
        if let Some(layer) = load_layer(path) {
            layers.push(layer);
        }
    }
    
    // Layer 3: Project layer (highest priority)
    if !config.sources.disable_project_layer {
        if let Some(project_layer) = load_project_layer() {
            layers.push(project_layer);
        }
    }
    
    layers
}
```

### 5.2 Asset Merge Algorithm

```rust
fn merge_assets(layers: &[PromptpackLayer]) -> HashMap<AssetId, Asset> {
    let mut merged = HashMap::new();
    
    // Process layers from lowest to highest priority
    for layer in layers {
        for asset in &layer.assets {
            // Higher priority completely replaces lower priority
            merged.insert(asset.id.clone(), asset.clone());
        }
    }
    
    merged
}
```

### 5.3 Directory Structure

```
~/.calvin/
├── .promptpack/                 # User-level promptpack
│   ├── config.toml
│   ├── policies/
│   │   └── my-coding-style.md
│   ├── actions/
│   │   └── my-review.md
│   └── agents/
│       └── my-helper.md
└── config.toml                  # Calvin user config (existing)

# Note: ~/.config/calvin/config.toml is the XDG-compliant location
# ~/.calvin/config.toml can be an alternative for simplicity
```

### 5.4 Error Handling

| Scenario | Behavior |
|----------|----------|
| 用户层路径不存在 | 静默跳过，不报错 |
| 额外层路径不存在 | 警告，继续执行 |
| 项目层不存在 | **有其他层时**：警告，继续执行；**无任何层时**：错误 |
| 同一层内 id 冲突 | 错误，报告冲突文件 |
| 跨层 id 覆盖 | 正常，verbose 模式下提示 |
| 循环引用（如 symlink） | 检测并报错 |
| 符号链接的 layer 路径 | 跟随符号链接，记录解析后的真实路径 |
| Layer 目录存在但为空 | 视为有效层，0 assets |

### 5.5 Asset Layer Migration Detection

当 asset 从一个层移动到另一个层时的处理：

```
场景：
1. 第一次 deploy：asset "review" 来自 project 层 → 输出 .claude/commands/review.md
2. 用户删除了 project 层的 review.md，但 user 层有同名 asset
3. 第二次 deploy：asset "review" 现在来自 user 层

处理：
- 输出路径相同 → 更新 lockfile 的 source_layer，不产生 orphan
- 输出路径不同 → 旧路径成为 orphan，新路径是新文件
```

**Lockfile 更新逻辑**：

```rust
fn detect_layer_migration(old_lockfile: &Lockfile, new_outputs: &[Output]) -> Vec<Migration> {
    let mut migrations = Vec::new();
    
    for output in new_outputs {
        if let Some(old_entry) = old_lockfile.get(&output.key) {
            if old_entry.source_layer != output.source_layer {
                migrations.push(Migration {
                    key: output.key.clone(),
                    old_layer: old_entry.source_layer.clone(),
                    new_layer: output.source_layer.clone(),
                });
            }
        }
    }
    
    migrations
}
```

### 5.6 Symlink Handling

**策略**：跟随符号链接，但记录原始路径用于显示

```rust
fn resolve_layer_path(path: &Path) -> Result<(PathBuf, PathBuf), Error> {
    let canonical = path.canonicalize()?;
    
    // 检测循环链接
    let mut seen = HashSet::new();
    let mut current = path.to_path_buf();
    while current.is_symlink() {
        if !seen.insert(current.clone()) {
            return Err(Error::CircularSymlink(path.to_path_buf()));
        }
        current = current.read_link()?;
    }
    
    Ok((path.to_path_buf(), canonical))
}
```

---

## 6. Migration & Compatibility

### 6.1 Backward Compatibility

- 现有项目无需任何改动
- 如果 `~/.calvin/.promptpack` 不存在，行为与当前完全一致
- 所有新配置项都有合理默认值

### 6.2 Migration Path

1. **Phase 1**: 实现用户层自动检测（`~/.calvin/.promptpack`）
2. **Phase 2**: 实现配置文件中的 `additional_layers`
3. **Phase 3**: 实现 `calvin layers` 命令和 `--layer` 标志

---

## 7. Implementation Plan

### Phase 0: Lockfile Migration (Breaking Change, Required First)

**Scope:**
- [ ] 迁移 lockfile 到项目根目录 `./calvin.lock`
- [ ] 实现向后兼容的自动迁移逻辑
- [ ] 扩展 lockfile 格式支持 `source_layer`, `source_asset`, `source_path`
- [ ] 更新所有读写 lockfile 的代码
- [ ] 添加迁移测试

**Estimated Effort:** 2-3 days

### Phase 1: Core Layer System (MVP)

**Scope:**
- [ ] 实现 `~/.calvin/.promptpack` 用户层检测
- [ ] 实现基本的 layer 合并逻辑
- [ ] 更新 `deploy` 命令支持多层来源
- [ ] 添加 verbose 模式的 layer 信息输出

**Estimated Effort:** 3-4 days

### Phase 2: Global Registry

**Scope:**
- [ ] 实现 `~/.calvin/registry.toml` 注册表
- [ ] 每次 deploy 自动注册项目
- [ ] 实现 `calvin projects` 命令
- [ ] 实现 `calvin projects --prune` 清理失效条目
- [ ] 实现 `calvin clean --all` 批量清理

**Estimated Effort:** 2-3 days

### Phase 3: Configuration & CLI

**Scope:**
- [ ] 扩展配置 schema 支持 `[sources]` section
- [ ] 实现 `--layer` CLI 标志
- [ ] 实现 `--no-user-layer` 和 `--no-additional-layers`
- [ ] 实现 `calvin init --user` 命令

**Estimated Effort:** 2-3 days

### Phase 4: Visibility & Tooling

**Scope:**
- [ ] 实现 `calvin layers` 命令
- [ ] 更新 `calvin check` 支持多层验证
- [ ] 更新文档

**Estimated Effort:** 1-2 days

### Total Estimated Effort: 10-15 days

---

## 8. Security Considerations

1. **路径验证**: 所有外部 layer 路径必须经过安全验证，防止路径遍历攻击
2. **符号链接**: 检测循环符号链接，防止无限循环
3. **权限检查**: 验证 layer 目录的读权限
4. **配置来源**: 不允许 project layer 添加任意外部路径（只能禁用，不能新增）

---

## 9. Lockfile Architecture (Breaking Change)

### 9.1 Problem with Current Design

当前 lockfile 位于 `.promptpack/.calvin.lock`，在多层场景下存在问题：
- 每个 layer 都有自己的 `.promptpack/`，lockfile 应该放在哪个？
- 违背行业惯例（npm, Cargo, Poetry 都把 lockfile 放在项目根目录）

### 9.2 Industry Best Practice

| Tool | Lockfile Location | Rationale |
|------|-------------------|-----------|
| npm | `./package-lock.json` | 项目根目录 |
| Cargo | `./Cargo.lock` | 项目根目录 |
| Poetry | `./poetry.lock` | 项目根目录 |
| Bundler | `./Gemfile.lock` | 项目根目录 |
| Terraform | `./.terraform.lock.hcl` | 项目根目录 |

**共同原则**：Lockfile 追踪**输出状态**，不是输入配置的一部分，应放在项目根目录。

### 9.3 New Lockfile Design

**迁移路径**：

```
# 旧位置（废弃）
.promptpack/.calvin.lock

# 新位置
./calvin.lock
```

**新格式（扩展）**：

```toml
# ./calvin.lock

version = 1

[files."project:.claude/commands/review.md"]
hash = "sha256:abc123..."
source_layer = "user"                              # 新增：来源层
source_asset = "review"                            # 新增：来源 asset id
source_path = "~/.calvin/.promptpack/actions/review.md"  # 新增：原始路径

[files."project:.cursor/commands/deploy.md"]
hash = "sha256:def456..."
source_layer = "project"
source_asset = "deploy"
source_path = ".promptpack/actions/deploy.md"
```

**向后兼容迁移**：

```rust
fn load_lockfile(project_root: &Path) -> Lockfile {
    let new_path = project_root.join("calvin.lock");
    let old_path = project_root.join(".promptpack/.calvin.lock");
    
    if new_path.exists() {
        return load_from(&new_path);
    }
    
    if old_path.exists() {
        let lockfile = load_from(&old_path);
        // Auto-migrate to new location
        save_to(&lockfile, &new_path);
        // Remove old file
        remove_file(&old_path);
        eprintln!("Migrated lockfile to {}", new_path.display());
        return lockfile;
    }
    
    Lockfile::new()
}
```

---

## 10. Output Provenance Tracking

### 10.1 Motivation

用户需要知道：
1. 每个输出文件来自哪个 `.promptpack`
2. 来自哪个 asset
3. 是否有被覆盖（override）

### 10.2 Lockfile 中的来源追踪

**Lockfile 格式**：
```toml
# ./calvin.lock

version = 1

[files."project:.claude/commands/review.md"]
hash = "sha256:abc123..."
source_layer = "user"                                  # 来源层名称
source_layer_path = "~/.calvin/.promptpack"            # 来源层实际路径
source_asset = "review"                                # 来源 asset id
source_file = "~/.calvin/.promptpack/actions/review.md"  # 来源文件

[files."project:.cursor/rules/style/RULE.md"]
hash = "sha256:def456..."
source_layer = "project"
source_layer_path = "./.promptpack"
source_asset = "code-style"
source_file = "./.promptpack/policies/code-style.md"
overrides = "user"                                     # 覆盖了哪个层的同名 asset
```

### 10.3 Provenance Report Command

```bash
# 查看所有输出的来源
calvin provenance

# 输出示例：
# Output Provenance Report
# ========================
#
# .claude/commands/review.md
#   Source: ~/.calvin/.promptpack/actions/review.md
#   Layer:  user (~/.calvin/.promptpack)
#   Asset:  review
#
# .cursor/rules/style/RULE.md
#   Source: ./.promptpack/policies/code-style.md
#   Layer:  project
#   Asset:  code-style
#   Note:   Overrides 'code-style' from user layer

# JSON 输出
calvin provenance --json
```

### 10.4 Verbose Deploy Output

```bash
$ calvin deploy -v

ℹ Layer Stack:
  3. [project] ./.promptpack/ (3 assets)
  2. [team]    ~/work/team-prompts/.promptpack (8 assets)
  1. [user]    ~/.calvin/.promptpack (5 assets)

ℹ Asset Provenance:
  • review        ← user:~/.calvin/.promptpack/actions/review.md
  • deploy        ← project:./.promptpack/actions/deploy.md
  • code-style    ← project:./.promptpack/policies/code-style.md
                    (overrides user layer)
  • security      ← user:~/.calvin/.promptpack/policies/security.md

✓ Compiled 4 assets from 2 layers
✓ Deployed to: claude-code, cursor
```

### 10.5 Data Structure

```rust
/// 输出文件的来源信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputProvenance {
    /// 来源层名称（user, project, custom）
    pub source_layer: String,
    /// 来源层的实际路径
    pub source_layer_path: PathBuf,
    /// 来源 asset id
    pub source_asset: String,
    /// 来源文件路径
    pub source_file: PathBuf,
    /// 如果覆盖了其他层的同名 asset，记录被覆盖的层
    pub overrides: Option<String>,
}
```

---

## 11. Global Registry Design

### 11.1 Motivation

用户需要：
1. 查看所有 calvin 管理的项目
2. 批量操作（如 `calvin clean --all`）
3. 检查所有项目的部署状态

### 11.2 Registry File

```toml
# ~/.calvin/registry.toml

version = 1

[[projects]]
path = "/Users/me/project-a"
lockfile = "/Users/me/project-a/calvin.lock"
last_deployed = "2024-12-24T10:00:00Z"
asset_count = 12

[[projects]]
path = "/Users/me/project-b"
lockfile = "/Users/me/project-b/calvin.lock"
last_deployed = "2024-12-24T11:30:00Z"
asset_count = 8
```

### 11.3 Registry Management

**自动注册**：每次 `calvin deploy` 成功后，自动更新 registry

```rust
fn register_project(project_path: &Path, lockfile_path: &Path, asset_count: usize) {
    let registry_path = home_dir().join(".calvin/registry.toml");
    let mut registry = load_registry(&registry_path);
    
    registry.upsert(ProjectEntry {
        path: project_path.to_path_buf(),
        lockfile: lockfile_path.to_path_buf(),
        last_deployed: Utc::now(),
        asset_count,
    });
    
    save_registry(&registry, &registry_path);
}
```

**清理失效条目**：项目被删除时的处理

```rust
fn cleanup_registry() {
    let registry = load_registry();
    let valid_projects: Vec<_> = registry.projects
        .into_iter()
        .filter(|p| p.lockfile.exists())
        .collect();
    
    if valid_projects.len() != registry.projects.len() {
        save_registry(&Registry { projects: valid_projects });
    }
}
```

### 11.4 New Commands

```bash
# 列出所有 calvin 管理的项目
calvin projects

# 输出示例：
# Calvin-managed Projects:
# ┌─────────────────────────────────┬────────────┬──────────────────┐
# │ Path                            │ Assets     │ Last Deployed    │
# ├─────────────────────────────────┼────────────┼──────────────────┤
# │ /Users/me/project-a             │ 12         │ 2h ago           │
# │ /Users/me/project-b             │ 8          │ 1d ago           │
# └─────────────────────────────────┴────────────┴──────────────────┘

# 清理所有项目的部署文件
calvin clean --all
calvin clean --all --dry-run

# 检查所有项目状态
calvin check --all

# 从 registry 中移除不存在的项目
calvin projects --prune
```

### 11.5 Behavior Matrix

| Command | Scope | Registry Required |
|---------|-------|-------------------|
| `calvin deploy` | 当前项目 | 写入 registry |
| `calvin clean` | 当前项目 | 读取当前 lockfile |
| `calvin clean --all` | 所有项目 | 读取 registry |
| `calvin projects` | 全局 | 读取 registry |
| `calvin projects --prune` | 全局 | 读写 registry |

---

## 12. Interactive UI Design

### 12.1 Layer Selection View

当使用 `calvin deploy` (interactive mode) 时，显示层级信息：

```
╭─────────────────────────────────────────────────────────────────╮
│  🚀 Calvin Deploy                                               │
│                                                                 │
│  Layer Stack (priority: high → low)                            │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │  3. [project] ./.promptpack/          5 assets            │ │
│  │  2. [team]    ~/team/.promptpack      8 assets            │ │
│  │  1. [user]    ~/.calvin/.promptpack   12 assets           │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                 │
│  Merged: 18 assets (7 from user, 6 from team, 5 from project)  │
│  Overrides: 7 assets                                            │
╰─────────────────────────────────────────────────────────────────╯
```

### 12.2 Override Confirmation

当高优先级层覆盖低优先级层时，在 verbose 模式下显示：

```
ℹ Asset Overrides:
  • code-style    project overrides user
  • security      project overrides team
  • review        team overrides user
```

### 12.3 Provenance Report View

`calvin provenance` 的输出设计：

```
╭─────────────────────────────────────────────────────────────────╮
│  📋 Output Provenance Report                                    │
╰─────────────────────────────────────────────────────────────────╯

.claude/commands/review.md
├─ Source: ~/.calvin/.promptpack/actions/review.md
├─ Layer:  user
└─ Asset:  review

.cursor/rules/style/RULE.md
├─ Source: ./.promptpack/policies/code-style.md
├─ Layer:  project
├─ Asset:  code-style
└─ Note:   Overrides 'code-style' from user layer

.github/copilot-instructions.md
├─ Source: ~/team/.promptpack/policies/guidelines.md
├─ Layer:  team
└─ Asset:  guidelines

Total: 15 output files from 3 layers
```

### 12.4 Projects List View

`calvin projects` 的输出设计：

```
╭─────────────────────────────────────────────────────────────────╮
│  📂 Calvin-managed Projects                                     │
╰─────────────────────────────────────────────────────────────────╯

┌──────────────────────────────────┬─────────┬──────────────────┐
│ Project                          │ Assets  │ Last Deployed    │
├──────────────────────────────────┼─────────┼──────────────────┤
│ ~/projects/webapp                │ 12      │ 2h ago           │
│ ~/projects/api-server            │ 8       │ 1d ago           │
│ ~/projects/mobile-app            │ 5       │ 3d ago           │
└──────────────────────────────────┴─────────┴──────────────────┘

Total: 3 projects, 25 assets
```

### 12.5 Layer Initialization View

`calvin init --user` 的输出设计：

```
╭─────────────────────────────────────────────────────────────────╮
│  ✓ User Layer Initialized                                      │
╰─────────────────────────────────────────────────────────────────╯

Created:
  ~/.calvin/.promptpack/
  ├── config.toml
  ├── policies/
  ├── actions/
  └── agents/

Next steps:
  1. Add your global prompts to ~/.calvin/.promptpack/
  2. Any project can now use these prompts
  3. Project-level .promptpack/ will override if needed
```

---

## 13. Error Handling Design

### 13.1 Error Design Principles (参考 src/error.rs)

所有错误消息必须包含：
1. **Clear description**: 明确说明什么出错了
2. **Suggestion**: 如何修复
3. **Documentation link**: 相关文档链接

### 13.2 Multi-Layer 相关错误类型

```rust
// 扩展 src/error.rs

/// Multi-layer specific errors
#[derive(Error, Debug)]
pub enum CalvinError {
    // ... 现有错误类型

    /// No layers found (no user layer, no additional layers, no project layer)
    #[error("no promptpack layers found\n  → Fix: Create a .promptpack/ directory or configure user layer\n  → Run: calvin init --user\n  → Docs: {}", docs::multi_layer_url())]
    NoLayersFound,

    /// Additional layer path not found (warning, not error)
    #[error("additional layer not found: {path}\n  → Fix: Check the path in your config or remove it\n  → Config: ~/.config/calvin/config.toml → sources.additional_layers")]
    AdditionalLayerNotFound { path: PathBuf },

    /// Circular symlink detected
    #[error("circular symlink detected in layer path: {path}\n  → Fix: Remove the circular symlink\n  → Symlink chain: {chain}")]
    CircularSymlink { path: PathBuf, chain: String },

    /// Layer path permission denied
    #[error("permission denied reading layer: {path}\n  → Fix: Check file permissions\n  → Run: chmod -R +r {path}")]
    LayerPermissionDenied { path: PathBuf },

    /// Duplicate asset ID within same layer
    #[error("duplicate asset ID '{id}' in layer '{layer}'\n  → Files:\n    1. {file1}\n    2. {file2}\n  → Fix: Rename one of the assets to have a unique ID")]
    DuplicateAssetInLayer {
        id: String,
        layer: String,
        file1: PathBuf,
        file2: PathBuf,
    },

    /// Invalid layer path (not a directory)
    #[error("layer path is not a directory: {path}\n  → Fix: Ensure the path points to a .promptpack directory")]
    InvalidLayerPath { path: PathBuf },

    /// User layer path not configured but required
    #[error("user layer path not configured\n  → Fix: Add to ~/.config/calvin/config.toml:\n    [sources]\n    user_layer_path = \"~/.calvin/.promptpack\"\n  → Or run: calvin init --user")]
    UserLayerNotConfigured,

    /// Registry file corrupted
    #[error("registry file corrupted: {path}\n  → Fix: Delete and rebuild registry\n  → Run: rm {path} && calvin deploy")]
    RegistryCorrupted { path: PathBuf },

    /// Lockfile format incompatible
    #[error("lockfile format incompatible (version {found}, expected {expected})\n  → Fix: Run calvin migrate to update\n  → Run: calvin migrate --lockfile")]
    LockfileVersionMismatch { found: u32, expected: u32 },
}
```

### 13.3 Warning Types (非致命错误)

```rust
/// Multi-layer specific warnings
pub enum LayerWarning {
    /// Additional layer not found (continue with other layers)
    AdditionalLayerMissing { path: PathBuf },
    
    /// Asset overridden by higher priority layer
    AssetOverridden {
        asset_id: String,
        by_layer: String,
        from_layer: String,
    },
    
    /// Layer has no assets
    EmptyLayer { path: PathBuf },
    
    /// Symlink followed
    SymlinkFollowed { original: PathBuf, resolved: PathBuf },
    
    /// Project layer not found (using user layer only)
    NoProjectLayer { using_layer: PathBuf },
}
```

### 13.4 Error Rendering

```rust
// src/ui/blocks/error.rs 扩展

/// Render a layer-related error with context
pub fn render_layer_error(
    error: &CalvinError,
    layers: &[Layer],
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut b = Box::with_style(BoxStyle::Error);
    
    // Error message
    b.add_line(format!("{} {}", Icon::Error.colored(supports_color, supports_unicode), error));
    b.add_empty();
    
    // Layer context
    if !layers.is_empty() {
        b.add_line("Layer Stack:");
        for (i, layer) in layers.iter().enumerate() {
            b.add_line(format!("  {}. [{}] {} ({} assets)", 
                layers.len() - i,
                layer.name,
                layer.path.display(),
                layer.assets.len()
            ));
        }
    }
    
    b.render(supports_color, supports_unicode)
}
```

### 13.5 Non-Silent Failure Policy

**原则**：除非明确预期，否则不静默失败

| 情况 | 行为 | 原因 |
|------|------|------|
| Additional layer 不存在 | 警告 + 继续 | 用户配置可能过时 |
| User layer 不存在 | 静默跳过 | 可能未配置，是正常情况 |
| Project layer 不存在 | 警告 + 使用其他层 | 用户可能只想用全局层 |
| 没有任何 layer | **错误** | 无法继续 |
| 符号链接 | 提示 + 继续 | 用户应该知道实际路径 |
| 循环符号链接 | **错误** | 无法解析 |
| Asset ID 冲突 (同层) | **错误** | 需要用户修复 |
| Asset ID 冲突 (跨层) | 提示覆盖 | 正常行为 |
| 权限问题 | **错误** | 用户需要修复 |

---

## 14. Edge Cases & Considerations

### 11.1 Version Compatibility

**向前兼容**：新版本 Calvin 读取旧格式 lockfile

```rust
fn load_lockfile_entry(entry: &TomlEntry) -> LockfileEntry {
    LockfileEntry {
        hash: entry.hash.clone(),
        // 旧格式没有这些字段时使用默认值
        source_layer: entry.source_layer.clone().unwrap_or("unknown".to_string()),
        source_asset: entry.source_asset.clone().unwrap_or("unknown".to_string()),
        source_path: entry.source_path.clone().unwrap_or("unknown".to_string()),
    }
}
```

**向后兼容**：旧版本 Calvin 读取新格式 lockfile
- 旧版本会忽略不认识的字段（TOML 的默认行为）
- 只要 `hash` 字段存在，旧版本就能正常工作
- 迁移时应提示用户升级

### 11.2 Config Merge Rules

**明确规则**：高层级**完全覆盖**低层级的同名 section

```toml
# user 层 ~/.calvin/.promptpack/config.toml
[targets]
enabled = ["claude-code", "cursor"]

[security]
mode = "balanced"

# project 层 ./.promptpack/config.toml
[targets]
enabled = ["vscode"]  # 完全覆盖 user 层的 targets

# 最终配置：
# targets.enabled = ["vscode"]  (来自 project)
# security.mode = "balanced"    (来自 user，因为 project 没有定义)
```

**不做深合并的原因**：
- 保持行为可预测
- 避免"我不知道这个配置从哪里来"的困惑
- 符合 Git config、ESLint 等工具的行为

### 11.3 Remote Deploy Behavior

```bash
calvin deploy --remote user@host:/path
```

**规则**：
- 只使用**项目层**（即 `.promptpack/` 目录）
- 忽略本地的用户层和额外层
- 远程机器上不检测 `~/.calvin/.promptpack`

**原因**：
- Remote deploy 的目的是"把这个项目部署到远程"
- 远程机器的用户层可能不存在或不同
- 保持行为简单可预测

**如果需要使用远程用户层**（未来版本）：
```bash
calvin deploy --remote user@host:/path --use-remote-user-layer
```

### 11.4 Watch Mode with Multi-Layer

**默认行为**：只监听项目层

```bash
calvin watch
# 监听 ./.promptpack/ 的变化
# 不监听 ~/.calvin/.promptpack 或其他层
```

**扩展选项**：

```bash
# 监听所有层
calvin watch --watch-all-layers

# 问题：如果全局层变化，所有正在运行的 watch 都会重新编译
# 这可能导致大量 I/O 和 CPU 使用
```

**建议**：
- 默认不监听全局层（用户需要手动重新 deploy）
- 提供 `--watch-all-layers` 选项供高级用户使用
- 在文档中说明性能影响

### 11.5 Environment Variable Priority

**优先级从低到高**：
1. 内置默认值
2. 用户层 config.toml
3. 额外层 config.toml（按配置顺序）
4. 项目层 config.toml
5. **环境变量**
6. CLI 参数

```bash
# 环境变量覆盖配置文件，但被 CLI 参数覆盖
CALVIN_SOURCES_USE_USER_LAYER=false calvin deploy
# 相当于
calvin deploy --no-user-layer
```

### 11.6 Registry Concurrency

**问题**：多个终端同时 deploy 不同项目

**解决方案**：使用文件锁

```rust
fn update_registry(project: &ProjectEntry) -> Result<(), Error> {
    let registry_path = home_dir().join(".calvin/registry.toml");
    let lock_path = home_dir().join(".calvin/registry.lock");
    
    // 获取独占锁，等待最多 5 秒
    let lock = FileLock::exclusive(&lock_path, Duration::from_secs(5))?;
    
    let mut registry = load_registry(&registry_path);
    registry.upsert(project);
    save_registry(&registry, &registry_path);
    
    drop(lock);  // 释放锁
    Ok(())
}
```

**备选方案**：原子写入 + 合并冲突检测
- 写入临时文件，然后原子重命名
- 如果文件被修改，重新读取并合并

### 11.7 Git Integration

**建议的 .gitignore 内容**：

```gitignore
# Calvin lockfile - SHOULD be committed (tracks deployed state)
# Do NOT add calvin.lock to .gitignore

# Calvin 生成的输出文件 - 可选是否提交
# .claude/
# .cursor/
# .github/copilot-instructions.md
```

**用户主目录**（`~/.calvin/`）：

```
~/.calvin/
├── .promptpack/      # 可选备份到 dotfiles repo
├── config.toml       # 可选备份到 dotfiles repo
└── registry.toml     # 不应该备份（本地状态，包含绝对路径）
```

### 11.8 Windows Path Handling

**`~` 展开规则**：

| OS | `~` expands to |
|----|----------------|
| macOS | `$HOME` (e.g., `/Users/username`) |
| Linux | `$HOME` (e.g., `/home/username`) |
| Windows | `%USERPROFILE%` (e.g., `C:\Users\username`) |

**路径分隔符**：

```rust
fn normalize_path_for_display(path: &Path) -> String {
    // 在 lockfile 和日志中统一使用正斜杠
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_path_for_fs(path: &Path) -> PathBuf {
    // 在文件系统操作中使用平台原生分隔符
    path.to_path_buf()
}
```

---

## 15. Open Questions

1. **MCP 配置合并**: 不同层都有 MCP server 定义时如何合并？
   - 建议：与 asset 相同规则，按 server name 作为 id，高层级覆盖

2. **Layer 继承链可视化**: 是否需要 `calvin explain <asset-id>` 显示 asset 的完整继承链？
   - 建议：作为 Phase 4 的可选功能

3. **Partial Override**: 是否支持 asset 的部分覆盖（只改某些字段）？
   - 建议：不支持，保持简单。完全覆盖更可预测

---

## 16. Success Metrics

1. 用户可以在 `~/.calvin/.promptpack` 维护个人 prompts，跨项目生效
2. 团队可以维护共享 promptpack，通过配置引入
3. 项目层可以覆盖任何上层定义
4. 不破坏现有单层使用场景
5. `calvin projects` 可以列出所有 calvin 管理的项目
6. `calvin clean --all` 可以批量清理所有项目
7. Windows/macOS/Linux 行为一致

---

## 17. References

- [Current Configuration Docs](../configuration.md)
- [Architecture Overview](../architecture/overview.md)
- [Original Spec](../../spec.md)

---

## Appendix A: Full Example

### User Setup

```bash
# 创建用户层 promptpack
calvin init --user

# 添加个人 coding style
cat > ~/.calvin/.promptpack/policies/my-style.md << 'EOF'
---
id: my-coding-style
title: My Coding Standards
kind: policy
scope: user
targets: [claude-code, cursor, vscode]
---

## My Personal Coding Standards

- Always use TypeScript strict mode
- Prefer functional programming patterns
- Never use `any` type
EOF
```

### Team Setup

```toml
# ~/.config/calvin/config.toml
[sources]
additional_layers = [
  "~/work/team-prompts/.promptpack",
]
```

### Project Override

```markdown
<!-- .promptpack/policies/my-style.md -->
---
id: my-coding-style  # Same ID overrides user layer
title: Project Specific Style
kind: policy
scope: project
targets: [claude-code, cursor]
---

## Project Coding Standards

This project uses JavaScript with JSDoc instead of TypeScript.
- Use JSDoc for type annotations
- ESLint with recommended rules
```

### Deploy Result

```bash
$ calvin deploy -v

ℹ Layer Stack:
  3. [project] ./.promptpack/ (3 assets)
  2. [team]    ~/work/team-prompts/.promptpack (8 assets)  
  1. [user]    ~/.calvin/.promptpack (5 assets)

ℹ Asset resolution:
  • my-coding-style: project layer overrides user layer

✓ Compiled 15 assets (1 overridden)
✓ Deployed to: claude-code, cursor, vscode
```

