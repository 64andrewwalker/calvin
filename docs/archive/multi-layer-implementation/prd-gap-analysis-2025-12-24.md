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

## 2. 仍需确认/仍有风险（建议发布前处理）

### R1. Interactive JSON state 命名语义（user_layer_only vs additional layers）

- 现状：
  - `calvin --json` 的 `state="user_layer_only"` 目前被用于“无项目 `.promptpack/`，但存在至少一个全局层（user/custom）”。
- 风险：
  - state 名称在语义上并不严格等同“只有 user layer”。
- 建议：
  - 若需要对外稳定语义，可新增更准确的 state（例如 `global_layers_only`），并保留 `user_layer_only` 作为兼容别名。

### R2. Remote deploy 与 `~` 路径

- PRD: §11.3 remote 只使用 project layer（已实现）；但 PRD 没有细化 “user-scope 输出” 的远程语义。
- 风险点：
  - 如果某些 asset 生成 `~/.xxx/...` 输出，远程实现中 `~` 由于被单引号包裹可能不会展开，导致在远程创建字面量目录 `~`。
- 建议：
  - 明确策略并加测试：
    - 选项 A：remote mode 强制所有输出为 project-scope（拒绝/改写 user-scope）
    - 选项 B：在远程侧显式展开 `~`（需谨慎，考虑不同 shell/权限）

### R3. `calvin diff` / `calvin watch` 的 project root 推导

- PRD: lockfile 与项目根的关系是核心约束（§9），命令应尽量避免从 source 推导 root。
- 现状：
  - Deploy 已修复为显式 `project_root`。
  - Diff/Watch 仍存在 “从 source.parent 推导 root” 的代码路径（未在本轮修复范围）。
- 建议：
  - 逐步对齐：引入统一的 `ProjectContext { project_root, project_layer_path }`，避免各命令分散推导。

### R4. 配置合并语义（是否需要按 promptpack layer 合并）

- PRD: §11.2 明确 “section 级覆盖，不做深合并”
- 现状：
  - 当前配置加载对 user config + project config 使用递归深合并（`merge_toml`）。
  - promptpack layer 的 config.toml 是否需要参与合并存在设计歧义（PRD §4.2 有描述，但实现未覆盖）。
- 建议：
  - 在发布前明确 “Config 的层次模型”：
    - 仅用户配置 + 项目配置（XDG user config + project `.promptpack/config.toml`）
    - 还是 user/custom/project promptpack 层的 `config.toml` 也参与合并
  - 明确后补齐测试用例（至少覆盖：同 section 多 key 时的覆盖行为）。

**最新进展（已修复一部分）**：
- 已实现 promptpack layer 的 section-level override 合并（PRD §11.2）：
  - 实现：`src/config/promptpack_layers.rs`
  - 覆盖测试：
    - deploy targets：`tests/cli_deploy_user_layer_targets_config.rs`
    - check/security：`tests/cli_check_respects_user_layer_promptpack_security_config.rs`
    - section override（非 deep merge）：`tests/config_promptpack_layer_merge_section_override.rs`
- 仍有缺口：
  - `merge_toml`（XDG user config + project config）仍是深合并；若 PRD 期望“对所有 config 来源都做 section 覆盖”，需要做一次明确的架构决策并补齐测试/文档。

## 3. 建议的发布前最小验收用例（手工 + 自动）

1. `~/.calvin/.promptpack/` 仅有 `config.toml`，无 `.md`：`calvin` 必须显示可 deploy 菜单，`calvin --json` 为 `user_layer_only` 且 `assets.total=0`
2. 在无 `.promptpack/` 的项目执行 `calvin deploy`：
   - 必须生成 `./calvin.lock`
   - `calvin clean` 必须能基于 lockfile 清理
3. `calvin deploy --source /path/to/external-pack`：
   - 产物写入当前项目
   - `./calvin.lock` 仍在当前项目
