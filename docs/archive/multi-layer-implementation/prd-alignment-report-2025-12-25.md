# Multi-Layer PRD Alignment Report (2025-12-25)

目标：逐项对齐 `docs/archive/multi-layer-implementation/multi-layer-promptpack-prd.md`，并用 **可验证证据**（测试/实现位置）确认发布前行为一致。

> 诚实优先：本报告只写“已验证”的结论；不确定/未覆盖的点会明确标注为风险或缺口。

## 0. 参考文档（已挖出）

- PRD: `docs/archive/multi-layer-implementation/multi-layer-promptpack-prd.md`
- Gap 分析（阶段性）: `docs/archive/multi-layer-implementation/prd-gap-analysis-2025-12-24.md`
- Bugfix 报告：
  - `docs/archive/multi-layer-implementation/bugfix-report-2025-12-24.md`
  - `docs/archive/multi-layer-implementation/bugfix-report-2025-12-25.md`
- 实施计划（用于核对范围与约束）:
  - `docs/archive/multi-layer-implementation/plan-00-lockfile-migration.md`
  - `docs/archive/multi-layer-implementation/plan-01-core-layer-system.md`
  - `docs/archive/multi-layer-implementation/plan-02-global-registry.md`
  - `docs/archive/multi-layer-implementation/plan-03-config-cli.md`
  - `docs/archive/multi-layer-implementation/plan-04-visibility-tooling.md`

## 1. 本轮验收方法

- 自动化：`cargo test`（全量）覆盖核心语义与回归用例（详见各条目 “证据”）。
- 关键回归均以 CLI E2E tests（`tests/cli_*.rs`）为主，避免只测内部实现。

## 2. PRD 核心设计对齐（按 PRD 章节）

### 4.1 Layer Hierarchy（user → custom → project）

- ✅ User layer：默认 `~/.calvin/.promptpack`，并支持 `sources.user_layer_path` 指向任意路径（含 `~` 展开）。
  - 证据：`tests/cli_interactive_user_layer_path_config.rs`
- ✅ Custom layers：来自 user config 的 `sources.additional_layers`，并支持 CLI `--layer` 追加（优先级按数组顺序递增）。
  - 证据：`tests/cli_deploy_layers_flags.rs`
- ✅ Project layer：默认 `./.promptpack/`，`--source` 仅替换 project layer 路径，不改变 project root。
  - 证据：`tests/cli_deploy_source_external.rs`

### 4.2 Asset Merge Semantics（同 ID 高层完全覆盖）

- ✅ 同 ID：高优先级 layer 完全覆盖低优先级 layer（无字段级合并）。
- ✅ 不同 ID：最终输出包含全部 assets。
  - 证据：由 `merge_layers`（domain service）实现；端到端由多层 deploy/watch/layers/provenance 的用例覆盖（见下方 5.x / 10.x）。

### 4.3 Configuration Schema（sources + ignore flags）

- ✅ 用户配置（XDG + legacy fallback）：
  - `~/.config/calvin/config.toml`（优先）
  - `~/.calvin/config.toml`（fallback）
  - 证据：`tests/cli_interactive_legacy_user_config_path.rs`
- ✅ `[sources]` 深合并语义（例外规则）：允许 project ignore flags 与 user paths 共存。
  - 证据：`src/config/loader.rs` 单测 `config_merge_sources_preserves_user_settings_when_project_sets_ignore_flags`
- ✅ 项目配置仅允许 ignore flags，禁止在 project config 中新增外部路径（安全约束）。
  - 证据：`src/config/loader.rs` 的 `validate_project_sources_config`（并有对应测试覆盖于 multi-layer suite）

### 4.4 CLI Interface（flags / commands）

- ✅ `calvin deploy --source` / `--layer` / `--no-user-layer` / `--no-additional-layers`
  - 证据：`tests/cli_deploy_layers_flags.rs`, `tests/cli_deploy_source_flag.rs`
- ✅ `calvin layers`（可 JSON）与 `calvin init --user`
  - 证据：`tests/cli_layers.rs`, `tests/cli_init_user.rs`
- ✅ `calvin --json` interactive state 语义修正：`global_layers_only` + 兼容 alias
  - 证据：`tests/cli_interactive_user_layer_empty.rs`, `tests/cli_interactive_additional_layers_only.rs`

## 2.5 PRD 详细设计对齐（5.x / 13.x 关键条款）

### 5.4 Error Handling（缺失 layer 的处理策略）

- ✅ User layer 不存在：静默跳过（不告警）。
- ✅ Additional layer 不存在：warning + continue。
- ✅ Project layer 不存在：warning + continue（当其他层存在时）。
- ✅ 无任何 layer：错误（无法继续）。
  - 证据：
    - `src/domain/services/layer_resolver.rs`（warn_on_missing 策略）
    - `tests/cli_no_layers_found.rs`

### 5.6 Symlink Handling（跟随 + 循环检测）

- ✅ 跟随符号链接，并记录 original/resolved path。
- ✅ 循环符号链接：错误。
  - 证据：`src/domain/services/layer_resolver/tests.rs`

### 13.5 Non-Silent Failure Policy（避免静默误行为）

- ✅ 对“额外层路径不存在”等用户可修复问题输出 warning。
- ✅ 对“无任何层”直接失败并给出修复提示（例如 `calvin init --user`）。
  - 证据：`tests/cli_no_layers_found.rs`

## 3. PRD 关键边界/高风险项（发布前必须稳定）

### 3.1 Remote `~` 展开（PRD §11.3 / §11.8）

- ✅ `calvin deploy --remote host:~/path` 不会因为 quoting 抑制 `~` 展开。
  - 证据：`tests/remote_tilde_expansion.rs`

### 3.2 `--source` 不污染 project_root（PRD §4.4 / §9）

- ✅ deploy：lockfile 固定写入项目根 `./calvin.lock`
  - 证据：`tests/cli_deploy_source_external.rs`
- ✅ watch：`project_root` 不再从 `source.parent()` 推导
  - 证据：`tests/cli_watch_source_external.rs`
- ✅ diff：`project_root` 不再从 `source.parent()` 推导
  - 证据：`tests/cli_diff_source_external.rs`

### 3.3 XDG user config + project config 合并语义（PRD §11.2）

- ✅ 顶层 section 原子覆盖（无深合并），仅 `[sources]` 例外深合并。
  - 证据：`src/config/loader.rs` 单测 `config_merge_section_override_drops_missing_keys`
- ✅ 空 section header（仅注释示例、无任何 key）不视为“定义了该 section”，不会覆盖低层配置。
  - 动机：模板/初始化文件经常包含 `[targets]` 等 header + 注释示例；若按 `contains_key` 处理会把低层有效配置覆盖回默认值（真实用户 P0）。
  - 证据：`tests/cli_deploy_empty_project_targets_section_does_not_override_user_layer.rs`；实现位于 `src/config/promptpack_layers.rs`

## 4. Lockfile / Provenance / Registry（PRD §9 / §10 / §11）

### 4.1 项目级 lockfile 迁移到 `./calvin.lock`（PRD §9）

- ✅ 旧 `.promptpack/.calvin.lock` 自动迁移到 `./calvin.lock`（并保留兼容读取）。
  - 证据：`tests/cli_lockfile_migration.rs`

### 4.2 Home deploy 的全局 lockfile（发布前 P0 修复）

- ✅ `deploy --home` / `clean --home` / `diff --home` 使用 `~/.calvin/calvin.lock`，避免污染当前目录与 registry。
  - 证据：`tests/cli_home_deploy_global_lockfile.rs`

### 4.3 输出来源追踪（Provenance）

- ✅ lockfile entry 记录来源 layer / 文件路径等，`calvin provenance` 可查询。
  - 证据：`tests/cli_provenance.rs`

### 4.4 全局 registry（projects / clean --all）

- ✅ 成功 deploy（project scope）写入 `~/.calvin/registry.toml`，并支持 `calvin projects` / `--prune`。
  - 证据：`tests/cli_projects.rs`
- ✅ `calvin clean --all` 走 registry 批量清理。
  - 证据：`tests/cli_clean_all.rs`
- ✅ 并发更新：registry 写入使用 `.lock` 文件锁。
  - 证据：`src/infrastructure/repositories/registry.rs`（`fs2::FileExt::lock_exclusive`）

## 5. Watch 行为（PRD §11.4）

- ✅ 默认只监听 project layer 的文件变化，但编译使用 resolved multi-layer stack。
  - 证据：`tests/cli_watch_includes_user_layer_assets.rs`
- ✅ `--watch-all-layers` 时监听所有层变更。
  - 证据：`tests/cli_watch_includes_user_layer_assets.rs`

## 5.5 Environment Variables（PRD §11.5 / §14.5）

- ✅ `CALVIN_TARGETS` 覆盖 config（并带校验/提示）。
- ✅ `CALVIN_SOURCES_USE_USER_LAYER` / `CALVIN_SOURCES_USER_LAYER_PATH` 覆盖 sources。
  - 证据：`src/config/loader.rs`（`with_env_overrides`）与 `src/config/tests.rs` 的 env 覆盖用例

## 5.6 Windows Path Handling（PRD §11.8）

- ✅ lockfile key 统一使用正斜杠；读取时做平台兼容处理。
  - 证据：`src/domain/entities/lockfile/tests.rs`

## 6. Diff 行为（补齐一致性，避免与 deploy/watch 偏离）

> PRD 未单列 diff，但 diff 必须与 deploy/watch 产物一致，否则会误导用户。

- ✅ diff 使用 resolved multi-layer stack 预览同样的输出，并且遵循 layer 合并后的 `[targets] enabled = [...]`。
  - 证据：`tests/cli_diff_includes_user_layer_assets.rs`

## 7. 偏差/缺口（必须诚实说明）

1. **交互式 UI 细节与 PRD mock 并非完全一致**：
   - PRD 里对 deploy 的 layer stack 展示使用 box + override 列表；当前实现偏向 CLI 打印与 MultiSelect（功能等价但 UI 形态不同）。
2. **PRD 的 Open Questions 未全部落地**（例如 MCP 合并语义），当前按“section override”处理。

## 8. 发布前建议的最小验收脚本

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```
