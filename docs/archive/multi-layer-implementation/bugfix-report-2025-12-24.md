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

### 1.6 Interactive 状态检测没有遵循 `sources.user_layer_path` / `sources.additional_layers`

**现象**：
- 用户在 `~/.config/calvin/config.toml` 配置：
  - `[sources] user_layer_path = ".../dotfiles/.promptpack"`（非默认路径）
  - 或仅配置 `[sources] additional_layers = [...]`
- 在一个没有 `.promptpack/` 的目录运行 `calvin --json` / `calvin`（interactive）会误判为 `no_promptpack`，从而进入初始化向导，而不是显示可部署菜单。

**PRD 期望**：
- PRD §4.3 / §5.1：layer resolver 应尊重 `sources.user_layer_path` 与 `sources.additional_layers`。
- PRD §13.5：存在任意 layer（即使为空）也应可继续。

**根因**：
- `src/commands/interactive/state.rs` 使用硬编码 `~/.calvin/.promptpack`，且完全忽略 additional layers。

**修复**：
- interactive state 复用 `LayerResolver`，用 config 的 sources 解析非项目层（user/custom）并统计 markdown 资产数。
- 新增覆盖测试：
  - `tests/cli_interactive_user_layer_path_config.rs`
  - `tests/cli_interactive_additional_layers_only.rs`

### 1.7 `calvin check` 未读取 user layer promptpack 的安全配置（`[security]`）

**现象**：
- 用户在 `~/.calvin/.promptpack/config.toml` 里配置安全选项（例如 `security.allow_naked = true`）。
- `calvin check` 仍按默认/项目配置执行，不会反映 user layer 的安全设置。

**PRD 期望**：
- PRD §11.2：promptpack layer 的 `config.toml` 参与合并；高层级 section 覆盖低层级 section。

**根因**：
- `run_doctor` 仅读取 `./.promptpack/config.toml`，且对缺失/错误配置会 `unwrap_or_default()`（静默回退）。

**修复（架构级）**：
- 增加 library 侧的分层 config 合并：`src/config/promptpack_layers.rs`
  - 对 top-level section 做 override（`format/security/targets/sync/output/mcp/deploy`；`sources` 例外不参与以避免循环依赖）
- `run_doctor` / `run_doctor_with_callback` 使用合并后的配置：`src/security/report.rs`
- 新增覆盖测试：`tests/cli_check_respects_user_layer_promptpack_security_config.rs`

### 1.8 `calvin init` 生成的 `config.toml` schema 过时（误导用户）

**现象**：
- `calvin init` 生成的 `.promptpack/config.toml` 仍包含历史字段（例如 `targets.include/exclude`、`deploy.default_scope`），与当前实现不一致，容易导致“看似配置了但不生效”的误判。

**修复**：
- 更新 init 模板为当前 schema：`targets.enabled`、有效的 `[sync]`、`[output]` 字段：`src/commands/init.rs`
- 新增覆盖测试：`tests/cli_init_project_config_template.rs`

### 1.9 Remote deploy 的 `~` 路径不展开（会创建字面量 `~` 目录）

**现象**：
- `calvin deploy --remote host:~/projects` 时，remote 侧的 SSH 命令会把路径包在单引号里（例如 `test -f '~/projects/file'`），导致 `~` **不会展开**，从而在远程产生字面量目录 `~` 或直接失败。

**PRD 期望**：
- PRD §11.8：支持 `~` 展开（远程场景至少不应因为 quoting 禁止 `~` 展开）。

**根因**：
- `src/infrastructure/sync/remote/mod.rs` 中远程路径拼接后直接做 `'...'` 单引号包裹，shell 的 tilde expansion 被抑制。

**修复**：
- RemoteDestination 在 remote_path 为 `~` / `~/...` 时，会通过一次 `ssh host "echo $HOME"` 获取远程 home（带缓存），并将 remote base path 展开为绝对路径后再进行安全引用。
- 新增回归测试（无需真实网络，PATH 注入 fake ssh）：`tests/remote_tilde_expansion.rs`

### 1.10 `calvin diff` / `calvin watch` 的 project_root 推导错误（`--source` 会污染输出根）

**现象**：
- 对外部 promptpack 使用 `--source /path/to/external/.promptpack` 时：
  - `calvin diff` 会把 project_root 当成 `source.parent()`，从而认为当前项目“所有文件都是 new”
  - `calvin watch` 会把输出和 `calvin.lock` 写到 external promptpack 的父目录，而不是当前项目目录

**PRD 期望**：
- PRD §4.4：`--source` 仅替换项目层路径，不应改变 project root。
- PRD §9：lockfile 固定在项目根目录 `./calvin.lock`。

**修复（架构级）**：
- DiffUseCase 统一使用显式 `options.project_root`（不再从 source 推导）：`src/application/diff.rs`
- CLI `calvin diff` / `calvin watch` 以 `std::env::current_dir()` 作为 project_root，并注入 options：
  - `src/commands/debug.rs`
  - `src/commands/watch.rs`
- WatchUseCase 使用 `WatchOptions.project_root` 而非 `source.parent()`：`src/application/watch/use_case.rs`
- 新增覆盖测试：
  - `tests/cli_diff_source_external.rs`
  - `tests/cli_watch_source_external.rs`

### 1.11 XDG user config + project config 的合并语义（从 deep merge 改为 section override）

**现象**：
- `~/.config/calvin/config.toml` 与 `./.promptpack/config.toml` 同时存在时，旧实现递归深合并：
  - user 的 `[security] allow_naked = true` 可能在 project 仅设置 `security.mode` 时被“残留”下来

**PRD 期望**：
- PRD §11.2：同名 section 高层级完全覆盖低层级（不做深合并）。

**修复**：
- `src/config/loader.rs`：对顶层 section 做原子覆盖（section-level override）。
- 例外：`[sources]` 保持深合并（项目 config 的 ignore flags 需要与用户配置的路径共存）。
- 新增单元测试（确保不回归）：
  - `src/config/loader.rs`（`config_merge_section_override_drops_missing_keys` / `config_merge_sources_preserves_user_settings_when_project_sets_ignore_flags`）

### 1.12 `sources.disable_project_layer` 未生效（仍然读取项目层）

**现象**：
- 用户在 user config（`~/.config/calvin/config.toml` 或 `~/.calvin/config.toml`）设置：
  - `[sources] disable_project_layer = true`
- 期望 deploy 仅使用 user/custom layers，但实际仍会读取 `./.promptpack/` 并参与合并/覆盖。

**PRD 期望**：
- PRD §4.3：`disable_project_layer` 可用于调试/特殊场景，禁用项目层。

**根因**：
- `disable_project_layer` 仅存在于 PRD/设计中，但没有贯穿到 layer resolver 与 deploy/watch/layers 的实际解析分支。

**修复**：
- `src/config/types.rs`：补齐 `SourcesConfig.disable_project_layer`（默认 false）。
- `src/domain/services/layer_resolver.rs`：支持 `with_disable_project_layer`，并在 local 模式下跳过 project layer（remote mode 仍强制使用 project layer）。
- `src/application/deploy/options.rs`：`DeployOptions` 增加 `use_project_layer` 并由上层注入。
- `src/commands/deploy/cmd.rs` / `src/commands/interactive/state.rs` / `src/application/layers/query.rs`：解析层级时尊重该开关。
- 新增覆盖测试：`tests/cli_deploy_disable_project_layer.rs`

### 1.13 支持 legacy 用户 config 路径 `~/.calvin/config.toml`

**现象**：
- PRD 在目录结构说明中提到 `~/.calvin/config.toml` 可作为简化选项，但实现只读取 XDG：
  - `~/.config/calvin/config.toml`

**PRD 期望**：
- PRD §5.3：`~/.config/calvin/config.toml`（XDG）为标准位置，同时支持 `~/.calvin/config.toml` 作为替代。

**修复**：
- `src/config/loader.rs`：
  - 优先读取 XDG user config；
  - 若 XDG 不存在，则回退读取 `~/.calvin/config.toml`（仅作为 fallback，不改变优先级层级）。
- 新增覆盖测试：`tests/cli_interactive_legacy_user_config_path.rs`

### 1.14 `calvin watch` 未按 multi-layer 编译 + 缺少 `--watch-all-layers`

**现象**：
- watch 初次 sync 只编译 `--source` 指向的单一目录，不会合并 user/custom layers：
  - user layer 的 assets 不会生成输出（尤其在“项目无任何 assets，但 user layer 有 assets”时很明显）。
- PRD 要求默认只监听项目层，但提供 `--watch-all-layers` 给高级用户；旧实现没有该选项，也不具备“watch 多层”的机制。

**PRD 期望**：
- PRD §11.4：
  - 默认：只监听 project layer（`./.promptpack`）变化
  - 但编译应使用 multi-layer 解析/合并语义（user/custom/project）
  - 可选：`calvin watch --watch-all-layers` 监听所有层

**根因（架构级）**：
- WatchUseCase 仍在使用 legacy `AssetPipeline` 对 `options.source` 做增量编译 → 天生只能看到单一目录，无法与 multi-layer 的 `LayerResolver` 对齐。

**修复（架构级）**：
- `src/commands/watch.rs`：
  - 使用 `merge_promptpack_layer_configs` 计算 watch 的有效配置（targets/deploy 等），并注入 `watch_all_layers`。
- `src/application/watch/use_case.rs`：
  - sync 逻辑改为直接执行 `DeployUseCase::execute`（multi-layer），确保编译/clean/orphans/registry 等行为与 deploy 完全一致；
  - 默认仅 watch project layer；`--watch-all-layers` 时通过 `LayerResolver` watch 所有 resolved layers；
  - watch 事件过滤扩展为 `.md` + `config.toml`，并确保删除文件也会触发 sync（用于 orphan 清理）。
- 新增覆盖测试：
  - `tests/cli_watch_includes_user_layer_assets.rs`（初次 sync 必须包含 user layer outputs）
  - `tests/cli_watch_includes_user_layer_assets.rs`（`--watch-all-layers` 时 user layer 变更触发重新部署）

## 2. 变更摘要（关键文件）

- `src/commands/interactive/state.rs`：interactive state 基于 `LayerResolver`（尊重 user_layer_path/additional_layers）
- `src/commands/interactive/menu.rs`：user-layer-only 菜单加入 Clean；并且 deploy 使用 `cwd/.promptpack` 作为项目层输入（而非把 user layer 当 project layer）
- `src/application/deploy/options.rs`：`DeployOptions.project_root`
- `src/application/deploy/use_case.rs`：文件系统访问统一走 “解析后的绝对路径”，避免 CWD 依赖
- `src/commands/deploy/cmd.rs`：project root 固定为当前目录
- `src/commands/clean.rs`：lockfile 定位与 project root 对齐（current_dir）
- `src/config/promptpack_layers.rs`：分层 promptpack `config.toml` 的 section-level override 合并
- `src/security/report.rs`：doctor/check 使用合并后的配置（不再只读项目 `.promptpack/config.toml`）
- `src/commands/deploy/layer_config.rs`：deploy targets 选择复用 `merge_promptpack_layer_configs`，并在 interactive 模式下做 targets 确认
- `src/commands/init.rs`：init 模板更新为当前 config schema（targets.enabled 等）
- `src/infrastructure/sync/remote/mod.rs`：remote `~` 展开（基于远程 `$HOME`，并缓存）
- `src/application/diff.rs`：diff 使用显式 `project_root`（不再从 source 推导）
- `src/commands/debug.rs`：diff 注入 `project_root = current_dir`
- `src/commands/watch.rs`：watch 注入 `project_root = current_dir`
- `src/application/watch/use_case.rs`：watch 以 `DeployUseCase` 执行 multi-layer sync；支持 `--watch-all-layers`
- `src/config/loader.rs`：XDG user config + project config 合并改为 section override（sources 例外深合并）
- `src/config/types.rs`：补齐 `sources.disable_project_layer`
- `src/domain/services/layer_resolver.rs`：支持禁用 project layer
- 新增测试：
  - `tests/cli_interactive_user_layer_empty.rs`
  - `tests/cli_interactive_user_layer_path_config.rs`
  - `tests/cli_interactive_additional_layers_only.rs`
  - `tests/cli_interactive_legacy_user_config_path.rs`
  - `tests/cli_deploy_source_external.rs`
  - `tests/cli_deploy_disable_project_layer.rs`
  - `tests/cli_deploy_user_layer_targets_config.rs`
  - `tests/cli_check_respects_user_layer_promptpack_security_config.rs`
  - `tests/cli_init_project_config_template.rs`
  - `tests/config_promptpack_layer_merge_section_override.rs`
  - `tests/cli_diff_source_external.rs`
  - `tests/cli_watch_source_external.rs`
  - `tests/cli_watch_includes_user_layer_assets.rs`
  - `tests/remote_tilde_expansion.rs`

## 3. 验证

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```
