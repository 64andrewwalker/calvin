# Phase 3: Configuration & CLI

> **Priority**: Medium  
> **Estimated Effort**: 2-3 days  
> **Dependencies**: Phase 1 complete
> **Architecture Reference**: `docs/architecture/layers.md`

## Objective

扩展配置系统支持 `[sources]` section，添加新的 CLI 参数。

## Architecture Compliance

按四层架构分布组件：

```
独立模块 (config/):
└── config/types.rs             # 扩展 SourcesConfig (~30 lines 新增)

Layer 0 (Presentation):
├── presentation/cli.rs         # 新增 CLI 参数 (~20 lines 新增)
├── commands/deploy.rs          # 集成新参数 (修改)
└── commands/init.rs            # 新增 --user 处理 (~80 lines)

Layer 2 (Domain):
└── domain/services/layer_resolver.rs  # 使用新配置 (修改)
```

**关键约束**:
- `SourcesConfig` 是纯数据类型，无业务逻辑
- CLI 参数解析在 Presentation 层
- `LayerResolver` 通过配置接收参数，不直接读取配置文件

## Detailed Tasks

### Task 3.1: Extend Config Types

**File**: `src/config/types.rs`

```rust
/// Sources configuration for multi-layer support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    /// 是否启用用户层
    #[serde(default = "default_true")]
    pub use_user_layer: bool,
    
    /// 用户层路径
    #[serde(default = "default_user_layer_path")]
    pub user_layer_path: PathBuf,
    
    /// 额外的层路径
    #[serde(default)]
    pub additional_layers: Vec<PathBuf>,
    
    /// 是否禁用项目层
    #[serde(default)]
    pub disable_project_layer: bool,
}

fn default_user_layer_path() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join(".calvin/.promptpack"))
        .unwrap_or_else(|| PathBuf::from("~/.calvin/.promptpack"))
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            use_user_layer: true,
            user_layer_path: default_user_layer_path(),
            additional_layers: Vec::new(),
            disable_project_layer: false,
        }
    }
}

/// 扩展 Config
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    // ... 现有字段
    
    #[serde(default)]
    pub sources: SourcesConfig,
}
```

**Tests**:
```rust
#[test]
fn parse_sources_config() {
    let content = r#"
        [sources]
        use_user_layer = true
        user_layer_path = "~/my-prompts/.promptpack"
        additional_layers = ["~/team/.promptpack"]
    "#;
    
    let config: Config = toml::from_str(content).unwrap();
    assert!(config.sources.use_user_layer);
    assert_eq!(config.sources.user_layer_path, PathBuf::from("~/my-prompts/.promptpack"));
    assert_eq!(config.sources.additional_layers.len(), 1);
}

#[test]
fn sources_config_defaults() {
    let config: Config = toml::from_str("").unwrap();
    assert!(config.sources.use_user_layer);
    assert!(config.sources.user_layer_path.to_string_lossy().contains(".calvin/.promptpack"));
}
```

### Task 3.2: Implement `--source` Parameter

**File**: `src/presentation/cli.rs`

```rust
#[derive(Parser)]
pub struct DeployArgs {
    // ... 现有参数
    
    /// Explicit path to .promptpack directory (overrides project layer detection)
    #[arg(long, value_name = "PATH")]
    pub source: Option<PathBuf>,
}
```

**File**: `src/commands/deploy.rs`

```rust
pub fn run_deploy(args: DeployArgs) -> Result<()> {
    let project_root = std::env::current_dir()?;
    
    // 确定源路径
    let source = args.source.unwrap_or_else(|| project_root.join(".promptpack"));
    
    // 构建层解析器
    let resolver = LayerResolver::new()
        .with_project_source(source)
        // ...
    
    // ...
}
```

### Task 3.3: Implement `--layer` Parameter

**File**: `src/presentation/cli.rs`

```rust
#[derive(Parser)]
pub struct DeployArgs {
    // ...
    
    /// Additional promptpack layers (can be specified multiple times)
    #[arg(long = "layer", value_name = "PATH")]
    pub layers: Vec<PathBuf>,
}
```

**Usage**:
```bash
calvin deploy --layer ~/team-prompts --layer ~/personal-prompts
```

### Task 3.4: Implement `--no-user-layer` and `--no-additional-layers`

**File**: `src/presentation/cli.rs`

```rust
#[derive(Parser)]
pub struct DeployArgs {
    // ...
    
    /// Disable user layer (~/.calvin/.promptpack)
    #[arg(long)]
    pub no_user_layer: bool,
    
    /// Disable additional configured layers
    #[arg(long)]
    pub no_additional_layers: bool,
}
```

**Integration**:
```rust
let resolver = LayerResolver::new()
    .with_user_layer_enabled(!args.no_user_layer && config.sources.use_user_layer)
    .with_additional_layers_enabled(!args.no_additional_layers)
    // ...
```

### Task 3.5: Implement `calvin init --user`

**File**: `src/commands/init.rs`

```rust
pub fn run_init_user() -> Result<()> {
    let user_layer = dirs::home_dir()
        .map(|h| h.join(".calvin/.promptpack"))
        .ok_or(CalvinError::UserLayerNotConfigured)?;
    
    if user_layer.exists() {
        eprintln!("User layer already exists: {}", user_layer.display());
        return Ok(());
    }
    
    // 创建目录结构
    fs::create_dir_all(user_layer.join("policies"))?;
    fs::create_dir_all(user_layer.join("actions"))?;
    fs::create_dir_all(user_layer.join("agents"))?;
    
    // 创建 config.toml
    let config_content = r#"# Calvin User Layer Configuration
# This is your personal promptpack that applies to all projects.

[format]
version = "1.0"
"#;
    fs::write(user_layer.join("config.toml"), config_content)?;
    
    // 输出成功信息
    render_init_user_success(&user_layer);
    
    Ok(())
}

fn render_init_user_success(path: &Path) {
    println!("╭─────────────────────────────────────────────────────────────────╮");
    println!("│  ✓ User Layer Initialized                                      │");
    println!("╰─────────────────────────────────────────────────────────────────╯");
    println!();
    println!("Created:");
    println!("  {}/", path.display());
    println!("  ├── config.toml");
    println!("  ├── policies/");
    println!("  ├── actions/");
    println!("  └── agents/");
    println!();
    println!("Next steps:");
    println!("  1. Add your global prompts to {}/", path.display());
    println!("  2. Any project can now use these prompts");
    println!("  3. Project-level .promptpack/ will override if needed");
}
```

**CLI Integration**:
```rust
// src/presentation/cli.rs
#[derive(Subcommand)]
pub enum Commands {
    Init {
        /// Initialize user-level promptpack (~/.calvin/.promptpack)
        #[arg(long)]
        user: bool,
    },
    // ...
}
```

### Task 3.6: Security Validation (PRD §8)

**约束**：项目配置不能添加外部层路径

**原因**：
- 防止恶意项目读取任意文件系统路径
- 项目只能禁用层，不能添加层
- 添加层必须在用户配置中

**File**: `src/config/loader.rs`

```rust
fn validate_project_config(config: &ProjectConfig) -> Result<(), ConfigError> {
    // 项目配置不允许添加 additional_layers
    if !config.sources.additional_layers.is_empty() {
        return Err(ConfigError::SecurityViolation {
            message: "Project config cannot add external layer paths. \
                      Use user config (~/.config/calvin/config.toml) instead.".to_string(),
            field: "sources.additional_layers".to_string(),
        });
    }
    
    // 项目配置不允许修改 user_layer_path
    if config.sources.user_layer_path.is_some() {
        return Err(ConfigError::SecurityViolation {
            message: "Project config cannot modify user layer path.".to_string(),
            field: "sources.user_layer_path".to_string(),
        });
    }
    
    Ok(())
}
```

**项目配置只允许**：
- `ignore_user_layer = true` - 禁用用户层
- `ignore_additional_layers = true` - 禁用额外层

**Tests**:
```rust
#[test]
fn project_config_cannot_add_layers() {
    let config = r#"
        [sources]
        additional_layers = ["~/malicious/.promptpack"]
    "#;
    
    let result = load_project_config(config);
    
    assert!(matches!(result, Err(ConfigError::SecurityViolation { .. })));
}

#[test]
fn project_config_can_disable_layers() {
    let config = r#"
        [sources]
        ignore_user_layer = true
        ignore_additional_layers = true
    "#;
    
    let result = load_project_config(config);
    assert!(result.is_ok());
}
```

## Verification

1. 运行 `cargo test config::sources`
2. 手动测试：
   - `calvin deploy --source ~/custom-prompts`
   - `calvin deploy --layer ~/team-prompts`
   - `calvin deploy --no-user-layer`
   - `calvin init --user`
3. 安全测试：
   - 项目配置添加 additional_layers 应报错

## Outputs

- `SourcesConfig` 类型
- `--source` 参数
- `--layer` 参数
- `--no-user-layer` 参数
- `--no-additional-layers` 参数
- `calvin init --user` 命令

