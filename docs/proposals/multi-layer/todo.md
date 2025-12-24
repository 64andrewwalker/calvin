# Multi-Layer PromptPack Implementation TODO

> Track implementation progress. Each phase must be completed with tests before moving to the next.
> 
> **UI 规范**: 所有新命令的 UI 必须遵循 `docs/ui-components-spec.md`

## Phase 0: Lockfile Migration

**Goal**: 将 lockfile 从 `.promptpack/.calvin.lock` 迁移到 `./calvin.lock`

### Tasks

- [ ] **0.1** 扩展 `LockfileEntry` 结构
  - [ ] 添加 `source_layer: Option<String>`
  - [ ] 添加 `source_layer_path: Option<PathBuf>`
  - [ ] 添加 `source_asset: Option<String>`
  - [ ] 添加 `source_file: Option<PathBuf>`
  - [ ] 添加 `overrides: Option<String>`
  - [ ] 添加单元测试

- [ ] **0.2** 更新 `TomlLockfileRepository` 序列化
  - [ ] 支持新字段的序列化/反序列化
  - [ ] 向后兼容：读取旧格式时使用默认值
  - [ ] 添加序列化测试

- [ ] **0.3** 更新 lockfile 路径逻辑
  - [ ] 修改 `get_lockfile_path()` 返回项目根目录
  - [ ] 实现自动迁移逻辑
  - [ ] 添加迁移测试

- [ ] **0.4** 更新所有使用 lockfile 的代码
  - [ ] `deploy` 命令
  - [ ] `clean` 命令
  - [ ] `diff` 命令
  - [ ] `watch` 命令

- [ ] **0.5** 端到端测试
  - [ ] 测试旧格式 lockfile 自动迁移
  - [ ] 测试新格式 lockfile 读写
  - [ ] 测试旧版本 Calvin 读取新格式（模拟）

**验收标准**:
- `cargo test lockfile` 全部通过
- 现有集成测试全部通过
- 手动测试 lockfile 迁移

---

## Phase 1: Core Layer System

**Goal**: 实现多层加载和合并的核心逻辑

### Tasks

- [ ] **1.1** 定义 `Layer` 和 `LayerSource` 类型
  - [ ] `src/domain/entities/layer.rs`
  - [ ] 包含 name, path, assets
  - [ ] 添加单元测试

- [ ] **1.2** 实现层解析逻辑
  - [ ] 检测用户层 `~/.calvin/.promptpack`
  - [ ] 检测项目层 `./.promptpack`
  - [ ] 返回有序层列表
  - [ ] 添加测试

- [ ] **1.3** 实现 asset 合并逻辑
  - [ ] `src/domain/services/layer_merger.rs`
  - [ ] 相同 ID 高层覆盖低层
  - [ ] 不同 ID 全部保留
  - [ ] 记录覆盖关系
  - [ ] 添加测试

- [ ] **1.4** 更新 `FsAssetRepository`
  - [ ] 支持从多个路径加载
  - [ ] 返回带层信息的 assets
  - [ ] 添加测试

- [ ] **1.5** 更新 `deploy` 命令
  - [ ] 使用新的层系统
  - [ ] verbose 模式显示层信息
  - [ ] 添加集成测试

**验收标准**:
- `calvin deploy` 能检测并使用用户层
- verbose 模式显示层栈
- 覆盖关系正确

---

## Phase 2: Global Registry

**Goal**: 实现全局项目追踪

### Tasks

- [ ] **2.1** 定义 `Registry` 和 `ProjectEntry` 类型
  - [ ] `src/infrastructure/repositories/registry.rs`
  - [ ] 添加测试

- [ ] **2.2** 实现 Registry 持久化
  - [ ] 读写 `~/.calvin/registry.toml`
  - [ ] 支持 upsert 和 prune
  - [ ] 添加测试

- [ ] **2.3** deploy 时自动注册
  - [ ] 成功后更新 registry
  - [ ] 添加测试

- [ ] **2.4** 实现 `calvin projects` 命令
  - [ ] 列出所有项目
  - [ ] 支持 `--prune` 清理失效
  - [ ] 添加 UI 渲染
  - [ ] 添加测试

- [ ] **2.5** 实现 `calvin clean --all`
  - [ ] 从 registry 读取所有项目
  - [ ] 批量清理
  - [ ] 添加测试

**验收标准**:
- `calvin projects` 显示所有项目
- `calvin clean --all` 正常工作
- Registry 自动更新

---

## Phase 3: Configuration & CLI

**Goal**: 配置和 CLI 扩展

### Tasks

- [ ] **3.1** 扩展 Config 支持 `[sources]`
  - [ ] `SourcesConfig` 类型
  - [ ] 默认值
  - [ ] 添加测试

- [ ] **3.2** 实现 `--source` 参数
  - [ ] 覆盖项目层检测
  - [ ] 添加测试

- [ ] **3.3** 实现 `--layer` 参数
  - [ ] 添加额外层
  - [ ] 可多次指定
  - [ ] 添加测试

- [ ] **3.4** 实现 `--no-user-layer` 和 `--no-additional-layers`
  - [ ] 禁用层
  - [ ] 添加测试

- [ ] **3.5** 实现 `calvin init --user`
  - [ ] 创建用户层目录
  - [ ] 添加测试

**验收标准**:
- 所有新 CLI 参数正常工作
- 配置文件解析正确

---

## Phase 4: Visibility & Tooling

**Goal**: 可视化和工具命令

### Tasks

- [ ] **4.1** 实现 `calvin layers` 命令
  - [ ] 显示层栈
  - [ ] 显示每层 asset 数量
  - [ ] 添加测试

- [ ] **4.2** 实现 `calvin provenance` 命令
  - [ ] 显示每个输出的来源
  - [ ] 支持 `--json`
  - [ ] 添加测试

- [ ] **4.3** 更新 `calvin check` 支持多层
  - [ ] 验证所有层
  - [ ] 检测冲突
  - [ ] 添加测试

- [ ] **4.4** 实现 `calvin migrate` 命令
  - [ ] 迁移 lockfile
  - [ ] 支持 `--dry-run`
  - [ ] 添加测试

- [ ] **4.5** 更新文档
  - [ ] `docs/configuration.md`
  - [ ] `docs/command-reference.md`
  - [ ] `CHANGELOG.md`

**验收标准**:
- 所有新命令正常工作
- 文档完整

---

## Error Handling

贯穿所有阶段：

- [ ] 添加 `NoLayersFound` 错误
- [ ] 添加 `AdditionalLayerNotFound` 警告
- [ ] 添加 `CircularSymlink` 错误
- [ ] 添加 `DuplicateAssetInLayer` 错误
- [ ] 添加 `LayerPermissionDenied` 错误
- [ ] 添加 `RegistryCorrupted` 错误
- [ ] 添加 `LockfileVersionMismatch` 错误

---

## Integration Tests

- [ ] 测试：只有项目层
- [ ] 测试：只有用户层
- [ ] 测试：项目层 + 用户层
- [ ] 测试：三层（用户 + 团队 + 项目）
- [ ] 测试：层覆盖
- [ ] 测试：lockfile 迁移
- [ ] 测试：registry 持久化
- [ ] 测试：`--source` 参数
- [ ] 测试：`--layer` 参数

---

## Progress Tracking

| Phase | Status | Start Date | End Date | Notes |
|-------|--------|------------|----------|-------|
| 0 | ⬜ Not Started | | | Lockfile migration |
| 1 | ⬜ Not Started | | | Core layer system |
| 2 | ⬜ Not Started | | | Global registry |
| 3 | ⬜ Not Started | | | Config & CLI |
| 4 | ⬜ Not Started | | | Visibility & tooling |

Legend: ⬜ Not Started | 🟡 In Progress | ✅ Complete

