# Multi-Layer Bugfix Report (2025-12-25)

本报告聚焦于 multi-layer 分支在发布前的高风险缺陷：`deploy --home` 的 lockfile / registry / clean 行为不一致，导致用户无法在交互式菜单中清理 home 下的已部署产物。

## 0. 参考文档（已挖出）

- PRD: `docs/archive/multi-layer-implementation/multi-layer-promptpack-prd.md`
- Gap 分析（上一轮）: `docs/archive/multi-layer-implementation/prd-gap-analysis-2025-12-24.md`
- Bugfix 报告（上一轮）: `docs/archive/multi-layer-implementation/bugfix-report-2025-12-24.md`
- 多层实现设计/计划（用于对齐意图与约束）:
  - `docs/archive/multi-layer-implementation/design.md`
  - `docs/archive/multi-layer-implementation/plan-00-lockfile-migration.md`
  - `docs/archive/multi-layer-implementation/plan-02-global-registry.md`
- CLI 行为参考:
  - `docs/command-reference.md`
  - `docs/cli-state-machine.md`

## 1. P0 缺陷：Home deploy 造成项目污染，且无法 clean

### 1.1 现象（用户报告可复现）

- 用户在任意目录执行 `calvin deploy --home`（全局部署到 `~/`）后：
  - 当前目录被创建 `./calvin.lock`（且可能是空内容）
  - `calvin projects` 将该目录注册为 “Calvin-managed project”（例如 assets=36）
  - 交互式菜单里的 clean 无法清理 home 下的 prompts（因为 lockfile/registry 语义错位）

### 1.2 期望（对齐产品语义）

- `deploy --home` 是 **全局部署**：
  - lockfile 应该落在用户状态目录，而不是当前项目目录
  - 不应污染 global registry（registry 用于 project 部署的批量管理）
- `clean --home` 应当在 **任意目录** 可用（不依赖当前项目存在 `.promptpack/`）

> 注意：multi-layer PRD 对 lockfile 的讨论主要聚焦 “项目级 lockfile 迁移到 `./calvin.lock`”。本条属于 scope/home 语义修复，但会影响用户对 multi-layer 的端到端体验（因为 user layer + home deploy 是常见组合）。

### 1.3 根因（架构级）

- DeployUseCase 在 Step 3 固定使用 `options.project_root` 解析 lockfile 路径，即使 `options.scope == User` 也会写到 `{cwd}/calvin.lock`。
- Deploy 成功后无条件注册 registry，导致 “home deploy 也被当成项目 deploy” 写入 `~/.calvin/registry.toml`。
- `calvin clean --home` 仍然用当前目录推导 lockfile（`resolve_lockfile_path(cwd, ...)`），因此在非项目目录无法找到 home 部署状态。

### 1.4 修复（端到端一致性）

**Lockfile 分层语义（关键修复）**

- Project scope：继续使用项目 lockfile（`{project_root}/calvin.lock`，并支持 legacy `.promptpack/.calvin.lock` 迁移）。
- Home/User scope：改为全局 lockfile（`{HOME}/.calvin/calvin.lock`）。

**Registry 语义**

- 仅在 `Scope::Project` 成功时注册项目到 registry。
- `deploy --home` 不再写入 registry，因此 `calvin projects` 不会被 home deploy 污染。

**Clean 语义**

- `calvin clean --home` 读取全局 lockfile（`{HOME}/.calvin/calvin.lock`），并可在任意目录运行。

**Diff 语义（补齐一致性）**

- `calvin diff --home` 使用全局 lockfile 参与冲突/修改检测（否则会把已跟踪文件当作 untracked）。

**交互式菜单**

- “Clean deployed files” 不再强制清理某个 scope：
  - 若同时存在 `./calvin.lock` 与 `~/.calvin/calvin.lock`，会先提示用户选择清理 **Project** 或 **Home**
  - 若只存在其中一个 lockfile，则直接进入该 scope 的 tree menu
  - 选择后进入原 `TreeMenu` 的多级选择（scope → target → files）

### 1.5 覆盖测试（端到端）

- 新增回归测试：`tests/cli_home_deploy_global_lockfile.rs`
  - `deploy --home` 不创建 `./calvin.lock`
  - 创建 `~/.calvin/calvin.lock`
  - 不写入 `~/.calvin/registry.toml`
  - `clean --home --yes` 在无 `.promptpack/` 的目录依然可清理并删除全局 lockfile

### 1.6 已污染环境的处理建议（面向用户）

如果你已经遇到 “某个目录被错误注册进 projects”，建议：

1. 删除该目录下的错误 `calvin.lock`（通常是空文件或仅包含 `[files]`）
2. 执行 `calvin projects --prune` 让 registry 清理失效条目

## 2. 变更摘要（关键文件）

- `src/application/lockfile_migration.rs`: 新增 `global_lockfile_path()`（`~/.calvin/calvin.lock`）
- `src/application/deploy/use_case.rs`: User scope 使用全局 lockfile；仅 Project scope 注册 registry
- `src/commands/clean.rs`: `clean --home` 使用全局 lockfile；`clean`（无 scope）在交互式 tree menu 前先做 project/home 选择
- `src/application/diff.rs`: User scope diff 使用全局 lockfile
- `src/commands/interactive/menu.rs`: clean 入口回归统一走 `calvin clean`（由 clean 命令自身处理 project/home 选择）
- `docs/command-reference.md`: 补齐 home deploy 的 lockfile 位置说明

## 3. 验证

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```
