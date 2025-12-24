# Multi-Layer Bugfix Report (2025-12-24)

本报告聚焦于 `docs/archive/multi-layer-implementation/multi-layer-promptpack-prd.md` 的关键行为，并对照当前实现做端到端验证与修复记录。

## 0. 参考文档（已挖出）

- PRD: `docs/archive/multi-layer-implementation/multi-layer-promptpack-prd.md`
- Design: `docs/archive/multi-layer-implementation/design.md`
- Handoff: `docs/archive/multi-layer-implementation/HANDOFF.md`
- Plans:
  - `docs/archive/multi-layer-implementation/plan-00-lockfile-migration.md`
  - `docs/archive/multi-layer-implementation/plan-01-core-layer-system.md`
  - `docs/archive/multi-layer-implementation/plan-02-global-registry.md`
  - `docs/archive/multi-layer-implementation/plan-03-config-cli.md`
  - `docs/archive/multi-layer-implementation/plan-04-visibility-tooling.md`

## 1. 用户已知问题（PRD 对齐）

### 1.1 用户层存在但“像没配置一样”（不会出现 Deploy 菜单）

**现象**：
- 用户在 `~/.calvin/.promptpack/` 有配置（例如只有 `config.toml`），进入一个没有 `.promptpack/` 的项目运行 `calvin`（interactive）时，会被判定为 “no_promptpack”，进入首次安装向导，而不是显示可部署菜单。

**PRD 期望**：
- PRD §5.4 / §13.5：`Layer 目录存在但为空 → 视为有效层，0 assets`。

**根因**：
- `src/commands/interactive/state.rs` 的状态检测逻辑把 “user layer” 判定为 `total_assets > 0` 才算存在，导致空目录被当成不存在。

**修复**：
- 空但存在的 user layer 也会返回 `ProjectState::UserLayerOnly(total=0)`。
- 新增覆盖测试：`tests/cli_interactive_user_layer_empty.rs`

### 1.2 User-layer-only 场景下没有 Clean

**现象**：
- 仅存在 user layer 时，interactive 菜单没有提供 `Clean`（只能在有项目 `.promptpack/` 的菜单里看到）。

**PRD 期望**：
- PRD §11.4 / §11.5：`calvin clean` 基于 lockfile 工作，应在用户层跨项目生效场景可用。

**修复**：
- `src/commands/interactive/menu.rs` 的 `interactive_user_layer_only` 菜单新增 “Clean deployed files” 入口，并调用 `calvin clean` 的交互式选择。

### 1.3 User-layer-only 场景下没有 lockfile（或 lockfile 在错误位置）

**现象**：
- interactive 的 “Deploy user layer to this project” 在某些情况下会把 lockfile 写到错误目录（例如 `~/.calvin/calvin.lock` 或外部 source 的父目录），导致当前项目缺失 `./calvin.lock`，进而无法 clean / provenance / projects 正常工作。

**PRD 期望**：
- PRD §9：lockfile 固定在项目根目录 `./calvin.lock`，与输入层（source/layers）解耦。

**根因（架构级）**：
1) Deploy 的 “project root” 被从 `source.parent()` 推导（对于 `--source` 外部目录/interactive 特殊分支会错）。  
2) 文件写入又依赖进程当前工作目录（相对路径），导致 “输出写到了当前项目，但 lockfile 却落在 source.parent()” 的不一致状态。

**修复（架构级）**：
- 引入明确的 `project_root` 概念（写入目录 + `calvin.lock` 所在目录）：
  - `src/application/deploy/options.rs`：`DeployOptions` 新增 `project_root` 字段
  - `src/application/deploy/use_case.rs`：所有文件 I/O（exists/read/hash/write/remove）对 project scope 输出使用 `project_root.join(relative_path)` 解析
  - `src/commands/deploy/cmd.rs`：CLI deploy 的 `project_root` 使用 `std::env::current_dir()`，不再从 `source` 推导
- 新增覆盖测试（修复 `--source` 语义与 lockfile 位置）：`tests/cli_deploy_source_external.rs`

### 1.4 Deploy 不遵循 user-layer 的 `[targets] enabled = [...]`（误部署到全部 targets）

**现象**：
- 用户在 `~/.calvin/.promptpack/config.toml` 设置：
  - `[targets] enabled = ["antigravity"]`
- 在任意项目执行 `calvin deploy`（未传 `--targets`）仍会部署到全部 targets。

**PRD 期望**：
- PRD §11.2：promptpack layer 的 `config.toml` 参与合并，且同名 section 按层级覆盖。
- 用户层的 `[targets]` 应影响最终部署目标（除非项目层显式覆盖）。

**根因**：
- 目标 target 选择只读取：
  - `~/.config/calvin/config.toml`（XDG user config）
  - `./.promptpack/config.toml`（项目 config）
- **不会读取** user layer promptpack 的 `~/.calvin/.promptpack/config.toml`，因此 `[targets]` 从未生效。

**修复**：
- Deploy 的 target 选择引入 “按 layer 解析并读取各层 `config.toml`”：
  - 从高优先级 → 低优先级查找 `[targets]` section（section-level override）
  - 若没有任何 layer 定义 `[targets]`，再回退到现有 config（用户 config / 默认值）
- 新增覆盖测试：`tests/cli_deploy_user_layer_targets_config.rs`

### 1.5 `calvin deploy` 在 interactive mode 需要确认 targets（避免“默认全量”误部署）

**现象**：
- `calvin deploy` 在 interactive 模式下不会询问要部署到哪些 targets，容易在未配置时直接部署到全部平台。

**修复**：
- interactive 模式下（TTY 且未 `--yes` 且未传 `--targets`），在 deploy 过程中弹出 targets 多选确认（默认选中配置/合并后的 targets）。

## 2. 变更摘要（关键文件）

- `src/commands/interactive/state.rs`：空 user layer 也算 layer
- `src/commands/interactive/menu.rs`：user-layer-only 菜单加入 Clean；并且 deploy 使用 `cwd/.promptpack` 作为项目层输入（而非把 user layer 当 project layer）
- `src/application/deploy/options.rs`：`DeployOptions.project_root`
- `src/application/deploy/use_case.rs`：文件系统访问统一走 “解析后的绝对路径”，避免 CWD 依赖
- `src/commands/deploy/cmd.rs`：project root 固定为当前目录
- `src/commands/clean.rs`：lockfile 定位与 project root 对齐（current_dir）
- `src/commands/deploy/layer_config.rs`：按 layer 读取 `config.toml` 的 `[targets]` 并在 interactive 模式下做 targets 确认
- 新增测试：
  - `tests/cli_interactive_user_layer_empty.rs`
  - `tests/cli_deploy_source_external.rs`
  - `tests/cli_deploy_user_layer_targets_config.rs`

## 3. 验证

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```
