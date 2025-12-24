# Multi-Layer PromptPack Implementation TODO

> Track implementation progress. Each phase must be completed with tests before moving to the next.
>
> **UI 规范**: 所有新命令的 UI 必须遵循 `docs/ui-components-spec.md`

## Phase 0: Lockfile Migration

**Goal**: 将 lockfile 从 `.promptpack/.calvin.lock` 迁移到 `./calvin.lock`

### Tasks

- [x] **0.1** 扩展 `LockfileEntry` 结构
  - [x] 添加 `source_layer: Option<String>`
  - [x] 添加 `source_layer_path: Option<PathBuf>`
  - [x] 添加 `source_asset: Option<String>`
  - [x] 添加 `source_file: Option<PathBuf>`
  - [x] 添加 `overrides: Option<String>`
  - [x] 添加单元测试

- [x] **0.2** 更新 `TomlLockfileRepository` 序列化
  - [x] 支持新字段的序列化/反序列化
  - [x] 向后兼容：读取旧格式时使用默认值
  - [x] 添加序列化测试

- [x] **0.3** 更新 lockfile 路径逻辑
  - [x] 修改 `get_lockfile_path()` 返回项目根目录
  - [x] 实现自动迁移逻辑
  - [x] 添加迁移测试

- [x] **0.4** 更新所有使用 lockfile 的代码
  - [x] `deploy` 命令
  - [x] `clean` 命令
  - [x] `diff` 命令
  - [x] `watch` 命令

- [x] **0.5** Windows 路径规范化
  - [x] lockfile 中统一使用正斜杠
  - [x] 读取时转换为平台原生分隔符
  - [x] 添加跨平台测试

- [x] **0.6** 端到端测试
  - [x] 测试旧格式 lockfile 自动迁移
  - [x] 测试新格式 lockfile 读写
  - [x] 测试旧版本 Calvin 读取新格式（模拟）
  - [x] 测试 Windows 路径处理

**验收标准**:

- `cargo test lockfile` 全部通过
- 现有集成测试全部通过
- 手动测试 lockfile 迁移

---

## Phase 1: Core Layer System

**Goal**: 实现多层加载和合并的核心逻辑

### Tasks

- [x] **1.1** 定义 `Layer` 和 `LayerSource` 类型
  - [x] `src/domain/entities/layer.rs`
  - [x] 包含 name, path, assets
  - [x] 添加单元测试

- [x] **1.2** 实现层解析逻辑
  - [x] 检测用户层 `~/.calvin/.promptpack`
  - [x] 检测项目层 `./.promptpack`
  - [x] 返回有序层列表
  - [x] 添加测试

- [x] **1.3** 实现 asset 合并逻辑
  - [x] `src/domain/services/layer_merger.rs`
  - [x] 相同 ID 高层覆盖低层
  - [x] 不同 ID 全部保留
  - [x] 记录覆盖关系
  - [x] 添加测试

- [x] **1.4** 定义 `LayerLoader` Port 和实现
  - [x] `src/domain/ports/layer_loader.rs`
  - [x] `src/infrastructure/layer/fs_loader.rs`
  - [x] 添加测试

- [x] **1.5** 更新 `deploy` 命令
  - [x] 使用新的层系统
  - [x] verbose 模式显示层信息
  - [x] 添加集成测试

- [x] **1.6** 处理 Asset 层迁移 (PRD §5.5)
  - [x] 检测 asset 从一个层移动到另一个层
  - [x] 更新 lockfile source_layer
  - [x] 添加测试

- [x] **1.7** 符号链接处理 (PRD §5.6)
  - [x] 跟随符号链接
  - [x] 检测循环符号链接
  - [x] 添加测试

**验收标准**:

- `calvin deploy` 能检测并使用用户层
- verbose 模式显示层栈
- 覆盖关系正确

---

## Phase 2: Global Registry

**Goal**: 实现全局项目追踪

### Tasks

- [ ] **2.1** 定义 `Registry` 和 `ProjectEntry` 类型
  - [ ] `src/domain/entities/registry.rs` (Entity 在 domain 层)
  - [ ] `src/domain/ports/registry_repository.rs` (Port 定义)
  - [ ] 添加测试

- [ ] **2.2** 实现 Registry 持久化
  - [ ] 读写 `~/.calvin/registry.toml`
  - [ ] 支持 upsert 和 prune
  - [ ] 添加测试

- [ ] **2.3** 创建 RegistryUseCase (Application 层)
  - [ ] `src/application/registry/use_case.rs`
  - [ ] 添加测试

- [ ] **2.4** deploy 时自动注册
  - [ ] 成功后更新 registry
  - [ ] 添加测试

- [ ] **2.5** 创建 Presentation 层文件
  - [ ] `src/commands/projects.rs`
  - [ ] `src/ui/views/projects.rs`

- [ ] **2.6** 实现 `calvin projects` 命令
  - [ ] 列出所有项目
  - [ ] 支持 `--prune` 清理失效
  - [ ] 添加 UI 渲染
  - [ ] 添加测试

- [ ] **2.7** 实现 `calvin clean --all`
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

- [ ] **3.6** 环境变量支持 (PRD §14.5)
  - [ ] `CALVIN_SOURCES_USE_USER_LAYER`
  - [ ] `CALVIN_SOURCES_USER_LAYER_PATH`
  - [ ] 添加测试

- [ ] **3.7** 安全验证 (PRD §8)
  - [ ] 项目配置不能添加 additional_layers
  - [ ] 项目配置不能修改 user_layer_path
  - [ ] 只允许禁用层
  - [ ] 添加测试

**验收标准**:

- 所有新 CLI 参数正常工作
- 配置文件解析正确
- 环境变量覆盖正确
- 安全验证通过

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

- [ ] **4.4** 实现 `calvin check --all`
  - [ ] 检查所有注册项目
  - [ ] 汇总结果
  - [ ] 添加测试

- [ ] **4.5** 添加 `--json` 输出支持
  - [ ] `calvin layers --json`
  - [ ] `calvin provenance --json`
  - [ ] 添加测试

- [ ] **4.6** 实现 `calvin migrate` 命令
  - [ ] 迁移 lockfile
  - [ ] 支持 `--dry-run`
  - [ ] 添加测试

- [ ] **4.7** 更新文档
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

- [x] 测试：只有项目层
- [x] 测试：只有用户层
- [x] 测试：项目层 + 用户层
- [ ] 测试：三层（用户 + 团队 + 项目）
- [x] 测试：层覆盖
- [x] 测试：lockfile 迁移
- [ ] 测试：registry 持久化
- [ ] 测试：`--source` 参数
- [ ] 测试：`--layer` 参数

---

## Progress Tracking

| Phase | Status | Start Date | End Date | Notes |
|-------|--------|------------|----------|-------|
| 0 | ✅ Complete | 2025-12-20 | 2025-12-24 | Lockfile migration |
| 1 | ✅ Complete | 2025-12-24 | 2025-12-24 | Core layer system |
| 2 | ⬜ Not Started | | | Global registry |
| 3 | ⬜ Not Started | | | Config & CLI |
| 4 | ⬜ Not Started | | | Visibility & tooling |

Legend: ⬜ Not Started | 🟡 In Progress | ✅ Complete
