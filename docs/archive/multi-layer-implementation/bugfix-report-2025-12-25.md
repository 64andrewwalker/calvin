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

---

## 4. P0 缺陷：无法临时覆盖 `deploy.target = "home"` 以部署到项目

### 4.1 现象（用户报告）

- 某些项目的 `.promptpack/config.toml` 中存在：
  - `[deploy] target = "home"`
- 用户在该项目内运行 `calvin deploy`（或使用 `-s` 指定该项目的 `.promptpack`）时，会始终部署到 `~/`。
- 用户希望“这一次”部署到项目目录，但不想修改/切换配置文件。

### 4.2 期望（对齐“显式优于隐式”）

- CLI 应允许用户显式表达“这次部署到项目”，并且 **不写回配置**：
  - `calvin deploy --project`

> 参考：`docs/archive/design/deploy-target-design.md`（override 不应修改 config）。

### 4.3 修复

- `calvin deploy` 增加 `--project` 旗标：
  - 与 `--home` / `--remote` 互斥
  - 优先级：`--remote` > `--home` > `--project` > `config.deploy.target` > 默认 project
- **不修改配置**：
  - `--project` / `--home` 作为 override，不会写回 `.promptpack/config.toml`
  - 仅当 `config.deploy.target == Unset` 且未使用 override 时，才会在成功 deploy 后写入默认目标（用于后续 `calvin watch`）

### 4.4 覆盖测试（端到端）

- 新增回归测试：`tests/cli_deploy_project_flag_overrides_home_config.rs`
  - 当项目配置 `target="home"` 时，`calvin deploy --project` 必须部署到项目目录
  - 且不会改写 config（仍保持 `target="home"`）

---

## 5. P0 缺陷：空 `[targets]` section 覆盖 user layer targets，导致误部署到全部 targets

### 5.1 现象（真实用户路径）

- 用户在 user layer promptpack 的 `~/.calvin/.promptpack/config.toml` 设置：
  - `[targets] enabled = ["antigravity"]`
- 项目层 `.promptpack/config.toml` 存在 **空的**：

```toml
[targets]
# enabled = ["cursor"]
```

- 在项目内执行 `calvin deploy`（未显式传 `--targets`）会把 targets 回退成默认 “全部 targets”，从而误写出 Cursor 等不期望的产物。

> 这在现实里很常见：`calvin init`/模板会生成 section header + 注释示例，TOML 解析后等价于一个空 table。

### 5.2 期望（对齐“可预测 + 不意外覆盖”）

- 保持 PRD §11.2 的 section-level override 语义：
  - **只有当某一层的 section table 含有至少一个 key** 时，才视为“该层定义了此 section”，从而覆盖低层。
  - 空 table（仅有 header/注释）应视为 “未定义该 section”，避免把低层有效配置覆盖回默认值。

### 5.3 根因（实现细节）

- `merge_promptpack_layer_configs` 用 `table.contains_key("targets")` 判断 section 是否存在。
- 即使 `[targets]` 是空 table，`contains_key("targets") == true`，从而把 user layer 的 `targets.enabled` 覆盖成 project 层的默认值（等价于 all targets）。

### 5.4 修复（架构级）

- `src/config/promptpack_layers.rs`：新增 `has_non_empty_table(...)`，并对 `format/security/targets/sync/output/mcp/deploy` 均改为：
  - **只有当对应 section 是非空 table 时** 才应用覆盖。

### 5.5 覆盖测试（端到端）

- 新增回归测试：`tests/cli_deploy_empty_project_targets_section_does_not_override_user_layer.rs`
  - user layer `enabled=["antigravity"]` + project 空 `[targets]` 时：
    - `.agent/...` 存在
    - `.cursor/...` 不应生成

---

## 6. P0 缺陷：`calvin diff` 与 deploy/watch 的 multi-layer + targets 语义不一致

### 6.1 现象

- 在仅有 user layer 或存在多层 stack 时：
  - `calvin deploy` / `calvin watch` 会基于 multi-layer stack 编译并写出产物
  - 但 `calvin diff` 仍以单一 source 预览，导致 diff 结果与实际 deploy 产物不一致（误导用户）

### 6.2 修复

- `src/application/diff.rs`：diff 走 `LayerResolver` + `merge_layers` 加载 multi-layer assets。
- `src/commands/debug.rs`：diff 的 targets 改为读取 layer-merged `config.toml`（`merge_promptpack_layer_configs`）。

### 6.3 覆盖测试（端到端）

- 新增回归测试：`tests/cli_diff_includes_user_layer_assets.rs`
  - 仅 user layer 存在时，`calvin diff --json` 必须包含 user layer 产物变更，并遵循 `enabled=["antigravity"]`（不应出现 `.cursor/`）。
