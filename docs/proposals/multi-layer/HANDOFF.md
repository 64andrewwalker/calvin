# Multi-Layer PromptPack Implementation Handoff

> 给高级软件工程师的实施指南

---

## 项目背景

Calvin 需要支持多层 promptpack 配置，允许用户从 `~/.calvin/.promptpack` (用户层) 和项目 `.promptpack/` (项目层) 合并 assets。

**核心文档**:
- PRD: `docs/proposals/multi-layer-promptpack-prd.md` (17 章节，已审核)
- 迁移指南: `docs/proposals/multi-layer-migration-guide.md`

---

## 实施顺序 (必须按顺序)

```
Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 4
```

### Phase 0: Lockfile Migration ⭐ **必须先完成**
- 将 lockfile 从 `.promptpack/.calvin.lock` 迁移到 `./calvin.lock`
- 扩展 lockfile 格式支持来源追踪
- **计划**: `docs/proposals/multi-layer/plan-00-lockfile-migration.md`
- **任务**: 6 个

### Phase 1: Core Layer System
- 实现层检测、合并逻辑
- **计划**: `docs/proposals/multi-layer/plan-01-core-layer-system.md`
- **任务**: 7 个 + 2 个 Notes

### Phase 2: Global Registry
- 实现项目注册和批量操作
- **计划**: `docs/proposals/multi-layer/plan-02-global-registry.md`
- **任务**: 7 个

### Phase 3: Configuration & CLI
- 扩展配置和 CLI 参数
- **计划**: `docs/proposals/multi-layer/plan-03-config-cli.md`
- **任务**: 7 个

### Phase 4: Visibility & Tooling
- 添加 `calvin layers`, `calvin provenance` 等命令
- **计划**: `docs/proposals/multi-layer/plan-04-visibility-tooling.md`
- **任务**: 7 个

---

## 开发方式: TDD (必须遵循)

### 每个任务的流程

```bash
# 1. 阅读任务规格 (在 plan-XX.md 中)

# 2. 写测试 (Red)
cargo test my_feature -- --nocapture
# 预期: 失败

# 3. 实现最小代码 (Green)
# ... 编码 ...
cargo test my_feature
# 预期: 通过

# 4. 重构 (Refactor)
cargo fmt
cargo clippy
cargo test
# 预期: 全部通过
```

### 每个 Task 完成后

1. **运行全部测试**: `cargo test`
2. **格式化**: `cargo fmt`
3. **Lint**: `cargo clippy`
4. **提交**: 一个任务一个 commit

---

## 架构约束 (必须遵守)

### 四层架构

```
Presentation → Application → Domain ← Infrastructure
```

### 文件位置 (参考 plan 文档中的 Architecture Compliance 部分)

| 类型 | 位置 | 示例 |
|------|------|------|
| Entity | `domain/entities/` | `layer.rs`, `registry.rs` |
| Service | `domain/services/` | `layer_resolver.rs`, `layer_merger.rs` |
| Port (trait) | `domain/ports/` | `layer_loader.rs`, `registry_repository.rs` |
| Infrastructure | `infrastructure/` | `repositories/registry.rs`, `layer/fs_loader.rs` |
| UseCase | `application/` | `registry/use_case.rs` |
| Command | `commands/` | `layers.rs`, `projects.rs` |
| View | `ui/views/` | `layers.rs`, `projects.rs` |

### 行数限制

- 每个文件 < 400 行
- 超过需要在文件顶部添加 `// calvin-no-split` 并说明理由

### 依赖规则

- Domain 层**不能**依赖 Infrastructure 或 Application
- 使用 Port (trait) 进行依赖反转

---

## 关键设计决策

| 决策 | 说明 |
|------|------|
| D1 | Lockfile 在项目根目录 (`./calvin.lock`) |
| D2 | 用户层路径可配置，默认 `~/.calvin/.promptpack` |
| D3 | 项目层不存在时，使用其他层并警告 |
| D4 | Lockfile 记录每个输出的来源 |
| D5 | 全局 Registry 在 `~/.calvin/registry.toml` |
| D6 | `--remote` 模式只使用项目层 |
| D7 | Watch 模式默认只监听项目层 |
| D8 | 项目配置只能禁用层，不能添加层 |

详见: `docs/proposals/multi-layer/design.md`

---

## 进度追踪

使用 `docs/proposals/multi-layer/todo.md` 追踪进度。

每完成一个任务:
1. 在 todo.md 中标记 `[x]`
2. 更新 Progress Tracking 表格

---

## 测试要求

### 单元测试

每个 plan 中的 Task 都有 Tests 部分，必须实现。

### 集成测试

在 `todo.md` 的 "Integration Tests" 部分列出，包括:
- 只有项目层
- 只有用户层
- 项目层 + 用户层
- 三层（用户 + 团队 + 项目）
- 层覆盖
- lockfile 迁移
- 等等

---

## UI 规范

所有新命令的 UI 必须遵循 `docs/ui-components-spec.md`:

- 使用 `CommandHeader`, `Box`, `ResultSummary` 等组件
- 使用 `src/ui/theme.rs` 中定义的颜色和图标
- 支持 ASCII 降级

---

## 完成标准

每个 Phase 完成时:
1. 所有任务的测试通过
2. `cargo test` 全部通过 (包括现有测试)
3. `cargo clippy` 无警告
4. 文档已更新 (Phase 4)

---

## 开始

```bash
# 1. 切换到功能分支
git checkout feature/multi-layer-promptpack

# 2. 阅读 Phase 0 计划
cat docs/proposals/multi-layer/plan-00-lockfile-migration.md

# 3. 从 Task 0.1 开始
# 先写测试，再实现

# 4. 提交
git add -A
git commit -m "feat(lockfile): extend LockfileEntry with provenance fields"
```

---

## 联系

如有疑问，参考:
- `docs/proposals/multi-layer/design.md` - 设计决策
- `docs/proposals/multi-layer-promptpack-prd.md` - 完整规格
- `docs/proposals/multi-layer/review-deep-check.md` - 审查清单

