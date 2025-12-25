# Multi-Layer PRD Gap Analysis (2025-12-24)

目标：对齐 `docs/archive/multi-layer-implementation/multi-layer-promptpack-prd.md`，用“可验证”的方式列出 **已对齐**、**已修复** 与 **仍存在风险/缺口** 的点。

> 本文件是 “诚实优先” 的风险清单：不把问题埋在代码里，也不把未知当成已完成。

## 1. 本轮已覆盖（有测试/有修复）

### A. 空 layer 仍应被识别

- PRD: “Layer 目录存在但为空 → 有效层（0 assets）”
- 覆盖：
  - interactive JSON：`tests/cli_interactive_user_layer_empty.rs`

### B. `--source` 不应改变项目根目录（lockfile 必须在项目根）

- PRD: §4.4 `--source` 仅替换 project layer 路径；§9 lockfile 固定 `./calvin.lock`
- 覆盖：
  - `tests/cli_deploy_source_external.rs`
- 关键修复：DeployUseCase 引入显式 `project_root`，消除 `source.parent()` 推导与 CWD 写入不一致。

### C. user-layer-only 场景可 clean

- PRD: §11.5 / §12：clean 基于 lockfile 与 registry 体系，应在跨项目场景可用
- 覆盖：
  - interactive menu 增加 clean 入口（人工验证为主）

### D. Interactive 尊重 sources 配置解析层（user_layer_path / additional_layers）

- PRD: §4.3 / §5.1 `sources.user_layer_path`、`sources.additional_layers`
- 覆盖：
  - user_layer_path（dotfiles 路径）：`tests/cli_interactive_user_layer_path_config.rs`
  - additional layers only：`tests/cli_interactive_additional_layers_only.rs`

### E. Remote deploy 的 `~` 路径展开

- PRD: §11.8 `~` expands to `$HOME`（远程场景至少不应被 quoting 抑制）
- 覆盖：
  - fake ssh 注入回归：`tests/remote_tilde_expansion.rs`
- 关键修复：RemoteDestination 对 `~` / `~/...` 通过 `ssh host "echo $HOME"` 展开为绝对路径（带缓存），避免 `'~'` 不展开的坑。

### F. `calvin diff` / `calvin watch` 的 project_root 不再从 source 推导

- PRD: §4.4 `--source` 不改变项目根；§9 lockfile 固定 `./calvin.lock`
- 覆盖：
  - diff：`tests/cli_diff_source_external.rs`
  - watch：`tests/cli_watch_source_external.rs`
- 关键修复：DiffUseCase/WatchUseCase 统一使用显式 `project_root`（CLI 以 current_dir 注入）。

### G. XDG user config + project config 合并语义（section override）

- PRD: §11.2 同名 section 高层级完全覆盖（不做深合并）
- 覆盖：
  - 单元测试：`src/config/loader.rs`（`config_merge_section_override_drops_missing_keys`）
- 备注：
  - `[sources]` 仍保持深合并（项目 ignore flags 与用户路径需要共存）。

### H. Watch 使用 multi-layer 编译 + `--watch-all-layers`

- PRD: §11.4
  - 默认只监听 project layer，但编译必须使用 resolved multi-layer stack（user/custom/project）
  - 提供 `--watch-all-layers` 给高级用户监听所有层
- 覆盖：
  - 初次 sync 包含 user layer assets：`tests/cli_watch_includes_user_layer_assets.rs`
  - `--watch-all-layers` 时 user layer 变更触发重新部署：`tests/cli_watch_includes_user_layer_assets.rs`

### I. Legacy user config 路径 `~/.calvin/config.toml`（fallback）

- PRD: §5.3 directory structure note
- 覆盖：
  - interactive 读取 legacy user config：`tests/cli_interactive_legacy_user_config_path.rs`

### J. `sources.disable_project_layer`

- PRD: §4.3
- 覆盖：
  - deploy 可以通过 user config 禁用 project layer：`tests/cli_deploy_disable_project_layer.rs`

### K. Interactive JSON state 命名语义（`global_layers_only` + alias）

- 现状（修复后）：
  - `calvin --json` 的 `state="global_layers_only"` 用于“无项目 `.promptpack/`，但存在至少一个全局层（user/custom）”。
  - 为兼容旧消费者：输出 `state_aliases=["user_layer_only"]`。
- 覆盖：
  - empty user layer：`tests/cli_interactive_user_layer_empty.rs`
  - additional layers only：`tests/cli_interactive_additional_layers_only.rs`
  - user_layer_path：`tests/cli_interactive_user_layer_path_config.rs`
  - legacy user config：`tests/cli_interactive_legacy_user_config_path.rs`

## 2. 仍需确认/仍有风险（建议发布前处理）

## 3. 建议的发布前最小验收用例（手工 + 自动）

1. `~/.calvin/.promptpack/` 仅有 `config.toml`，无 `.md`：`calvin` 必须显示可 deploy 菜单，`calvin --json` 为 `global_layers_only`，且 `state_aliases` 包含 `user_layer_only`；并且 `assets.total=0`
2. 在无 `.promptpack/` 的项目执行 `calvin deploy`：
   - 必须生成 `./calvin.lock`
   - `calvin clean` 必须能基于 lockfile 清理
3. `calvin deploy --source /path/to/external-pack`：
   - 产物写入当前项目
   - `./calvin.lock` 仍在当前项目
4. `calvin watch`：
   - 初次 sync 必须包含 user layer assets（即使默认不 watch user layer 变化）
   - `calvin watch --watch-all-layers` 修改 user layer 后必须触发重新部署
