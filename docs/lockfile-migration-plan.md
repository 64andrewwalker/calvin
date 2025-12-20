# Lockfile 迁移计划

> 日期: 2025-12-20
> 状态: 设计完成，准备实施

## 目标

将 `sync/lockfile.rs` 的功能合并到 `domain/entities/lockfile.rs`，消除重复代码。

## 当前状态

### sync/lockfile.rs (387 行)

```rust
pub enum LockfileNamespace { Project, Home }

pub struct Lockfile {
    version: u32,
    files: BTreeMap<String, FileEntry>,
}

pub struct FileEntry {
    pub hash: String,
}

// 方法
impl Lockfile {
    fn new()
    fn load_or_new<FS>(path, fs)  // I/O
    fn load<FS>(path, fs)          // I/O
    fn save<FS>(&self, path, fs)   // I/O
    fn get_hash(&self, path)
    fn set_hash(&mut self, path, hash)
    fn remove(&mut self, path)
    fn contains(&self, path)
    fn paths()
    fn get(&self, path)
    fn entries()
}

// 辅助函数
pub fn lockfile_key(namespace, path) -> String
pub fn hash_content(content: &[u8]) -> String
pub fn hash_file(path) -> String
```

### domain/entities/lockfile.rs (295 行)

```rust
pub struct LockfileEntry {
    hash: String,
}

pub struct Lockfile {
    version: u32,
    entries: BTreeMap<String, LockfileEntry>,
}

// 方法
impl Lockfile {
    fn new()
    fn version()
    fn is_empty()
    fn len()
    fn make_key(scope, path)  // 类似 lockfile_key
    fn parse_key(key)
    fn get(&self, key)
    fn get_hash(&self, key)
    fn set(&mut self, key, hash)
    fn remove(&mut self, key)
    fn keys()
    fn entries()
    fn keys_for_scope(scope)
}
```

## 差异分析

| 功能 | sync 版本 | domain 版本 | 迁移策略 |
|------|----------|-------------|----------|
| `LockfileNamespace` | ✅ 有 | ❌ 无 | 迁移到 domain |
| `lockfile_key()` | ✅ 函数 | ✅ `make_key()` | 已有 |
| `hash_content()` | ✅ 有 | ❌ 无 | 迁移到 domain 或 infrastructure |
| `load/save` | ✅ 在 struct 上 | ✅ 在 Repository | 使用 Repository |
| `set_hash()` | ✅ 有 | ❌ 用 `set()` | 添加别名或更新调用 |
| `contains()` | ✅ 有 | ❌ 无 | 添加到 domain |
| `paths()` | ✅ 有 | ✅ `keys()` | 已有 |

## 迁移步骤

### Step 1: 添加缺失的方法到 domain::entities::Lockfile

1.1. 添加 `contains(&self, key: &str) -> bool`
1.2. 添加 `set_hash(&mut self, key, hash)` 作为 `set()` 的别名

### Step 2: 迁移 LockfileNamespace 到 domain

2.1. 创建 `domain/value_objects/lockfile_namespace.rs`
2.2. 迁移 `LockfileNamespace` enum
2.3. 创建 `lockfile_key()` 函数

### Step 3: 迁移 hash_content 函数

3.1. 将 `hash_content()` 移到 `infrastructure/` 或 `domain/services/`
3.2. 更新所有调用点

### Step 4: 更新 sync 模块使用新类型

4.1. 更新 `sync/plan.rs` 使用 `domain::entities::Lockfile`
4.2. 更新 `sync/engine.rs`
4.3. 更新 `sync/orphan.rs`
4.4. 更新 `commands/debug.rs`

### Step 5: 删除 sync/lockfile.rs 中的重复代码

5.1. 删除 `Lockfile` struct（保留 re-export）
5.2. 删除 `FileEntry` struct
5.3. 保留 `lockfile_key()` 作为兼容性函数
5.4. 保留 `hash_content()` 直到完全迁移

### Step 6: 清理

6.1. 更新 `sync/mod.rs` 的导出
6.2. 更新文档
6.3. 运行全部测试

## TDD 测试计划

### Step 1 测试

```rust
#[test]
fn lockfile_contains_returns_true_for_existing_key() {
    let mut lockfile = Lockfile::new();
    lockfile.set("test:path", "hash");
    assert!(lockfile.contains("test:path"));
}

#[test]
fn lockfile_contains_returns_false_for_missing_key() {
    let lockfile = Lockfile::new();
    assert!(!lockfile.contains("test:path"));
}

#[test]
fn lockfile_set_hash_is_alias_for_set() {
    let mut lockfile = Lockfile::new();
    lockfile.set_hash("test:path", "hash");
    assert_eq!(lockfile.get_hash("test:path"), Some("hash"));
}
```

### Step 2 测试

```rust
#[test]
fn lockfile_namespace_project_as_str() {
    assert_eq!(LockfileNamespace::Project.as_str(), "project");
}

#[test]
fn lockfile_namespace_home_as_str() {
    assert_eq!(LockfileNamespace::Home.as_str(), "home");
}

#[test]
fn lockfile_key_project_path() {
    let key = lockfile_key(LockfileNamespace::Project, Path::new("file.md"));
    assert_eq!(key, "project:file.md");
}
```

## 验收标准

- [ ] `sync/lockfile.rs` 不再定义 `Lockfile` struct
- [ ] 所有 560+ 测试通过
- [ ] 无 deprecated 警告
- [ ] 代码行数减少 > 100 行

