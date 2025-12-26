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

- [x] **2.1** 定义 `Registry` 和 `ProjectEntry` 类型
  - [x] `src/domain/entities/registry.rs` (Entity 在 domain 层)
  - [x] `src/domain/ports/registry_repository.rs` (Port 定义)
  - [x] 添加测试

- [x] **2.2** 实现 Registry 持久化
  - [x] 读写 `~/.calvin/registry.toml`
  - [x] 支持 upsert 和 prune
  - [x] 添加测试

- [x] **2.3** 创建 RegistryUseCase (Application 层)
  - [x] `src/application/registry/use_case.rs`
  - [x] 添加测试

- [x] **2.4** deploy 时自动注册
  - [x] 成功后更新 registry
  - [x] 添加测试

- [x] **2.5** 创建 Presentation 层文件
  - [x] `src/commands/projects.rs`
  - [x] `src/ui/views/projects.rs`

- [x] **2.6** 实现 `calvin projects` 命令
  - [x] 列出所有项目
  - [x] 支持 `--prune` 清理失效
  - [x] 添加 UI 渲染
  - [x] 添加测试

- [x] **2.7** 实现 `calvin clean --all`
  - [x] 从 registry 读取所有项目
  - [x] 批量清理
  - [x] 添加测试

**验收标准**:

- `calvin projects` 显示所有项目
- `calvin clean --all` 正常工作
- Registry 自动更新

---

## Phase 3: Configuration & CLI

**Goal**: 配置和 CLI 扩展

### Tasks

- [x] **3.1** 扩展 Config 支持 `[sources]`
  - [x] `SourcesConfig` 类型
  - [x] 默认值
  - [x] 添加测试

- [x] **3.2** 实现 `--source` 参数
  - [x] 覆盖项目层检测
  - [x] 添加测试

- [x] **3.3** 实现 `--layer` 参数
  - [x] 添加额外层
  - [x] 可多次指定
  - [x] 添加测试

- [x] **3.4** 实现 `--no-user-layer` 和 `--no-additional-layers`
  - [x] 禁用层
  - [x] 添加测试

- [x] **3.5** 实现 `calvin init --user`
  - [x] 创建用户层目录
  - [x] 添加测试

- [x] **3.6** 环境变量支持 (PRD §14.5)
  - [x] `CALVIN_SOURCES_USE_USER_LAYER`
  - [x] `CALVIN_SOURCES_USER_LAYER_PATH`
  - [x] 添加测试

- [x] **3.7** 安全验证 (PRD §8)
  - [x] 项目配置不能添加 additional_layers
  - [x] 项目配置不能修改 user_layer_path
  - [x] 只允许禁用层
  - [x] 添加测试

**验收标准**:

- 所有新 CLI 参数正常工作
- 配置文件解析正确
- 环境变量覆盖正确
- 安全验证通过

---

## Phase 4: Visibility & Tooling

**Goal**: 可视化和工具命令

### Tasks

- [x] **4.1** 实现 `calvin layers` 命令
  - [x] 显示层栈
  - [x] 显示每层 asset 数量
  - [x] 添加测试

- [x] **4.2** 实现 `calvin provenance` 命令
  - [x] 显示每个输出的来源
  - [x] 支持 `--json`
  - [x] 添加测试

- [x] **4.3** 更新 `calvin check` 支持多层
  - [x] 验证所有层
  - [x] 检测冲突
  - [x] 添加测试

- [x] **4.4** 实现 `calvin check --all`
  - [x] 检查所有注册项目
  - [x] 汇总结果
  - [x] 添加测试

- [x] **4.5** 添加 `--json` 输出支持
  - [x] `calvin layers --json`
  - [x] `calvin provenance --json`
  - [x] 添加测试

- [x] **4.6** 实现 `calvin migrate` 命令
  - [x] 迁移 lockfile
  - [x] 支持 `--dry-run`
  - [x] 添加测试

- [x] **4.7** 更新文档
  - [x] `docs/configuration.md`
  - [x] `docs/command-reference.md`
  - [x] `CHANGELOG.md`

**验收标准**:

- 所有新命令正常工作
- 文档完整

---

## Error Handling

贯穿所有阶段：

- [x] 添加 `NoLayersFound` 错误
- [x] 添加 `AdditionalLayerNotFound` 警告
- [x] 添加 `CircularSymlink` 错误
- [x] 添加 `DuplicateAssetInLayer` 错误
- [x] 添加 `LayerPermissionDenied` 错误
- [x] 添加 `RegistryCorrupted` 错误
- [x] 添加 `LockfileVersionMismatch` 错误

---

## Integration Tests

- [x] 测试：只有项目层
- [x] 测试：只有用户层
- [x] 测试：项目层 + 用户层
- [x] 测试：三层（用户 + 团队 + 项目）
- [x] 测试：层覆盖
- [x] 测试：Asset 从项目层迁移到用户层（provenance 更新）
- [x] 测试：Asset 从用户层迁移到项目层（provenance 更新）
- [x] 测试：lockfile 迁移
- [x] 测试：registry 持久化
- [x] 测试：`--source` 参数
- [x] 测试：`--layer` 参数

---

## Progress Tracking

| Phase | Status | Start Date | End Date | Notes |
|-------|--------|------------|----------|-------|
| 0 | ✅ Complete | 2025-12-20 | 2025-12-24 | Lockfile migration |
| 1 | ✅ Complete | 2025-12-24 | 2025-12-24 | Core layer system |
| 2 | ✅ Complete | 2025-12-24 | 2025-12-24 | Global registry |
| 3 | ✅ Complete | 2025-12-24 | 2025-12-24 | Config & CLI |
| 4 | ✅ Complete | 2025-12-24 | 2025-12-24 | Visibility & tooling |

Legend: ⬜ Not Started | 🟡 In Progress | ✅ Complete
