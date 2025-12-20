# Sync 模块迁移计划

> TDD 驱动的接口迁移

---

## 需要迁移的接口

### 1. 核心类型

| 原类型 | 位置 | 使用者 | 迁移目标 |
|--------|------|--------|----------|
| `SyncResult` | sync/mod.rs | bridge.rs, interactive.rs | domain::value_objects |
| `ScopePolicy` | sync/scope.rs | cmd.rs, debug.rs | domain::policies |
| `SyncDestination` | sync/plan.rs | targets.rs | domain::ports (已有) |
| `Lockfile` | sync/lockfile.rs | debug.rs | domain::entities (已有) |
| `OrphanFile` | sync/orphan.rs | ui/views/orphan.rs | domain::entities |

### 2. 工具函数

| 函数 | 位置 | 使用者 | 迁移目标 |
|------|------|--------|----------|
| `expand_home_dir` | sync/mod.rs | debug.rs | infrastructure::fs |
| `lockfile_key` | sync/lockfile.rs | - | domain::entities::lockfile |
| `detect_orphans` | sync/orphan.rs | debug.rs | domain::services::orphan_detector |
| `compile_assets` | sync/compile.rs | - | domain::services::compiler |

### 3. 复杂依赖

| 模块 | 依赖深度 | 迁移优先级 |
|------|----------|------------|
| `SyncEngine` | 高 (watcher.rs) | 低 - 暂时保留 |
| `AssetPipeline` | 中 (debug.rs) | 中 |
| `InteractiveResolver` | 低 | 已迁移 |

---

## TDD 迁移步骤

### Step 1: SyncResult → DeployResult 转换

**目标**: 使用 DeployResult 替代 SyncResult

**测试**:
```rust
#[test]
fn deploy_result_to_sync_result_conversion() {
    let deploy = DeployResult { written: vec![PathBuf::from("a.md")], ... };
    let sync = SyncResult::from(deploy);
    assert_eq!(sync.written, vec!["a.md".to_string()]);
}
```

**实现**: 在 bridge.rs 中添加 From trait 实现

### Step 2: expand_home_dir → LocalFs

**目标**: 将 expand_home_dir 移到 infrastructure::fs

**测试**:
```rust
#[test]
fn local_fs_expands_home() {
    let fs = LocalFs::new();
    let expanded = fs.expand_home(Path::new("~/test"));
    assert!(expanded.to_string_lossy().contains("Users") || expanded.to_string_lossy().contains("home"));
}
```

**实现**: 已存在于 FileSystem trait

### Step 3: OrphanFile → domain::entities

**目标**: 将 OrphanFile 移到 domain 层

**位置**: `domain/entities/orphan.rs`

### Step 4: 更新 debug.rs 使用新 API

**目标**: 让 debug 命令使用新架构

**依赖**:
- Lockfile → 使用 domain::entities::Lockfile
- detect_orphans → 使用 domain::services::OrphanDetector

---

## 保留模块

以下模块暂时保留（watch 命令依赖）:

- `sync/engine.rs` - SyncEngine
- `sync/pipeline.rs` - AssetPipeline
- `sync/plan.rs` - 部分规划逻辑
- `sync/execute.rs` - 执行逻辑
- `sync/remote.rs` - rsync 逻辑

---

## 可删除模块

以下模块可以安全删除：

- `sync/compile.rs` - 已迁移到 domain/services/compiler
- `sync/tests.rs` - 随着其他模块删除

---

## 执行顺序

1. ✅ 确认 FileSystem::expand_home 可用
2. 🔲 将 SyncResult 转换逻辑封装为 From trait
3. 🔲 将 OrphanFile 迁移到 domain::entities
4. 🔲 更新 debug.rs 使用新 API
5. 🔲 删除不再使用的 sync 子模块

