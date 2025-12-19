# 跨平台兼容层设计

## 问题背景

Calvin 需要支持 macOS、Linux 和 Windows 三个主要平台。平台差异主要体现在：

1. **路径分隔符**: `/` (Unix) vs `\` (Windows)
2. **用户目录位置**: `$HOME` (Unix) vs `%USERPROFILE%` (Windows)
3. **可执行文件扩展名**: 无 (Unix) vs `.exe` (Windows)
4. **换行符**: `\n` (Unix) vs `\r\n` (Windows)
5. **文件权限**: Unix 有 chmod，Windows 使用 ACL
6. **符号链接**: Unix 原生支持，Windows 需要权限
7. **大小写敏感性**: Unix 敏感，Windows 不敏感

## 设计原则

### 1. 路径处理抽象层

**使用 `std::path::PathBuf` 而非字符串拼接**

```rust
// ❌ 错误做法
let path = format!("{}/{}", base, file);

// ✅ 正确做法
let path = base.join(file);
```

### 2. 平台特定配置

```rust
// domain/value_objects/platform.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    MacOS,
    Linux,
    Windows,
}

impl Platform {
    pub fn current() -> Self {
        #[cfg(target_os = "macos")]
        return Self::MacOS;
        
        #[cfg(target_os = "linux")]
        return Self::Linux;
        
        #[cfg(target_os = "windows")]
        return Self::Windows;
    }
    
    pub fn is_unix(&self) -> bool {
        matches!(self, Self::MacOS | Self::Linux)
    }
    
    pub fn line_ending(&self) -> &'static str {
        match self {
            Self::Windows => "\r\n",
            _ => "\n",
        }
    }
}
```

### 3. 用户目录抽象

```rust
// infrastructure/platform/home.rs

pub trait HomeDirectory {
    fn home_dir(&self) -> Option<PathBuf>;
    fn config_dir(&self) -> Option<PathBuf>;
}

pub struct SystemHomeDirectory;

impl HomeDirectory for SystemHomeDirectory {
    fn home_dir(&self) -> Option<PathBuf> {
        dirs::home_dir()
    }
    
    fn config_dir(&self) -> Option<PathBuf> {
        // macOS: ~/Library/Application Support
        // Linux: ~/.config
        // Windows: %APPDATA%
        dirs::config_dir()
    }
}
```

## 目标路径的平台适配

### 不同 AI 工具的配置位置

Calvin 支持 5 个目标平台，每个平台有不同的配置目录结构：

#### Claude Code

| Scope | macOS/Linux | Windows |
|-------|-------------|---------|
| Project | `.claude/commands/<id>.md` | `.claude\commands\<id>.md` |
| User | `~/.claude/commands/<id>.md` | `%USERPROFILE%\.claude\commands\<id>.md` |

**附加文件**:
- `.claude/settings.json` - 安全设置（deny list）

#### Cursor

| Scope | macOS/Linux | Windows |
|-------|-------------|---------|
| Project | `.cursor/rules/<id>/RULE.md` | `.cursor\rules\<id>\RULE.md` |
| User | `~/.cursor/rules/<id>/RULE.md` | `%USERPROFILE%\.cursor\rules\<id>\RULE.md` |

**注意**: Cursor 自动读取 Claude 的 commands 目录，无需单独生成 commands。

#### VS Code (GitHub Copilot)

| Scope | macOS/Linux | Windows |
|-------|-------------|---------|
| Project | `.github/instructions/<id>.instructions.md` | `.github\instructions\<id>.instructions.md` |
| User | N/A (project-only) | N/A |

**附加文件**:
- `.github/copilot-instructions.md` - 合并模式（可选）
- `AGENTS.md` - 索引文件

#### Codex (OpenAI)

| Scope | macOS/Linux | Windows |
|-------|-------------|---------|
| Project | `.codex/prompts/<id>.md` | `.codex\prompts\<id>.md` |
| User | `~/.codex/prompts/<id>.md` | `%USERPROFILE%\.codex\prompts\<id>.md` |

**格式**: 使用 YAML frontmatter，包含 `$ARGUMENTS` 占位符

#### Antigravity (Google Gemini)

| Scope | macOS/Linux | Windows |
|-------|-------------|---------|
| Project | `.agent/workflows/<id>.md` | `.agent\workflows\<id>.md` |
| User | `~/.gemini/antigravity/global_workflows/<id>.md` | `%USERPROFILE%\.gemini\antigravity\global_workflows\<id>.md` |

### 路径处理的关键点

1. **使用 `~` 前缀**: 所有 User scope 路径使用 `~/` 前缀，在 sync 时展开
2. **PathBuf::join()**: 自动处理平台分隔符
3. **User scope 在 Windows**: `~` 展开为 `%USERPROFILE%` (通常是 `C:\Users\<name>`)

### 平台感知路径生成器

```rust
// domain/services/path_resolver.rs

pub struct PathResolver {
    platform: Platform,
    home_dir: PathBuf,
}

impl PathResolver {
    pub fn new(platform: Platform, home_dir: PathBuf) -> Self {
        Self { platform, home_dir }
    }
    
    /// Expand ~ to home directory
    pub fn expand_home(&self, path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        if path_str.starts_with("~/") || path_str == "~" {
            let relative = path_str.strip_prefix("~/").unwrap_or("");
            self.home_dir.join(relative)
        } else {
            path.to_path_buf()
        }
    }
    
    /// Get target-specific user config directory
    pub fn user_config_dir(&self, target: Target) -> PathBuf {
        match target {
            Target::ClaudeCode => self.home_dir.join(".claude"),
            Target::Cursor => self.home_dir.join(".cursor"),
            Target::VSCode => {
                if self.platform == Platform::Windows {
                    // Use %APPDATA%\Code on Windows
                    dirs::data_dir()
                        .map(|d| d.join("Code"))
                        .unwrap_or_else(|| self.home_dir.join(".vscode"))
                } else {
                    self.home_dir.join(".vscode")
                }
            }
            Target::Codex => self.home_dir.join(".codex"),
            _ => self.home_dir.clone(),
        }
    }
}
```

## 文件系统操作的平台差异

### 1. 原子写入

```rust
// infrastructure/fs/atomic_write.rs

pub fn atomic_write(path: &Path, content: &str) -> Result<()> {
    let dir = path.parent().ok_or_else(|| anyhow!("Invalid path"))?;
    
    // Create temp file in same directory
    let temp = tempfile::NamedTempFile::new_in(dir)?;
    
    // Write content
    temp.as_file().write_all(content.as_bytes())?;
    
    // Platform-specific rename handling
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Set reasonable permissions (644)
        std::fs::set_permissions(temp.path(), std::fs::Permissions::from_mode(0o644))?;
    }
    
    // Persist (atomic rename)
    temp.persist(path)?;
    
    Ok(())
}
```

### 2. 文件权限

```rust
// infrastructure/fs/permissions.rs

pub fn set_readonly(path: &Path, readonly: bool) -> Result<()> {
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_readonly(readonly);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(unix)]
pub fn set_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    let mode = perms.mode() | 0o111; // Add execute bits
    perms.set_mode(mode);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(windows)]
pub fn set_executable(_path: &Path) -> Result<()> {
    // Windows determines executability by extension, not permissions
    Ok(())
}
```

## 换行符处理

### 设计决策：统一使用 LF

**理由**：
1. Git 默认将换行符规范化为 LF
2. 所有 AI 工具都能正确处理 LF
3. 简化跨平台协作

```rust
// domain/services/content_normalizer.rs

pub fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n")
}

/// For display purposes only, use platform-native endings
pub fn to_platform_endings(content: &str, platform: Platform) -> String {
    if platform == Platform::Windows {
        // Already has only LF, convert to CRLF
        content.replace('\n', "\r\n")
    } else {
        content.to_string()
    }
}
```

## SSH/Rsync 跨平台

### macOS/Linux

```bash
rsync -avz --delete source/ user@host:dest/
```

### Windows 考虑

1. **WSL (推荐)**: 在 WSL 环境中运行 Calvin
2. **Git Bash**: 包含 rsync
3. **Native Windows SSH**: Windows 10+ 自带 OpenSSH

```rust
// infrastructure/remote/rsync.rs

pub fn rsync_command(platform: Platform) -> &'static str {
    match platform {
        Platform::Windows => {
            // Try WSL first, fallback to Git Bash rsync
            if which::which("wsl").is_ok() {
                "wsl rsync"
            } else {
                "rsync" // Assume Git Bash or similar
            }
        }
        _ => "rsync",
    }
}
```

## 测试策略

### 1. 条件编译测试

```rust
#[test]
#[cfg(unix)]
fn test_unix_specific_behavior() {
    // Unix-specific test
}

#[test]
#[cfg(windows)]
fn test_windows_specific_behavior() {
    // Windows-specific test
}
```

### 2. 平台无关测试

```rust
#[test]
fn test_path_joining_is_platform_agnostic() {
    let base = PathBuf::from("project");
    let file = "config.toml";
    
    let result = base.join(file);
    
    // PathBuf handles separators automatically
    assert!(result.ends_with("config.toml"));
}
```

### 3. CI 矩阵

```yaml
# .github/workflows/ci.yml
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: cargo test --workspace
```

## 实现检查清单

- [ ] 将所有硬编码的 `/` 替换为 `Path::join()`
- [ ] 使用 `dirs` crate 获取系统目录
- [ ] 添加 `Platform` value object
- [ ] 创建 `PathResolver` 服务
- [ ] 实现跨平台的原子写入
- [ ] 添加 Windows CI 测试
- [ ] 文档化 Windows 用户的 rsync 要求
- [ ] 测试 WSL 环境兼容性

## 依赖建议

```toml
[dependencies]
dirs = "5"           # 跨平台目录查找
which = "6"          # 查找可执行文件
tempfile = "3"       # 跨平台临时文件
```

## 优先级

1. **P0 - 必须**: 路径处理、用户目录
2. **P1 - 重要**: 原子写入、文件权限
3. **P2 - 后续**: Windows SSH/rsync 支持、WSL 集成

