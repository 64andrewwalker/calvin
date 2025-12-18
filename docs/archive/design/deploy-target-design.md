# Calvin Deploy Target Design

> 设计原则：**显式优于隐式，用户意图优于便利性**

## 问题分析

### 当前问题

1. **隐式状态**: deploy `--home` 保存状态，watch 隐式读取 → 用户不知道会同步到哪里
2. **Magic行为**: 系统"记住"用户选择，但用户可能不知道这个机制存在
3. **不一致性**: deploy 需要显式 `--home`，但 watch 不需要
4. **认知负担**: 用户需要记住上次执行了什么命令

### 核心问题

> **系统在代替用户做决定，而用户没有意识到这一点**

---

## 设计原则

### 1. 不要代替用户做决定

```
❌ 错误: 记住用户上次选择，下次自动应用
✅ 正确: 每次都让用户明确表达意图
```

### 2. 配置是声明式的，不是运行时推断的

```
❌ 错误: .calvin-state.json 记录运行时状态
✅ 正确: config.toml 声明用户期望的行为
```

### 3. 缺少配置时，询问而不是假设

```
❌ 错误: 没有配置 → 使用默认值(project)
✅ 正确: 没有配置 → 交互式询问或报错
```

---

## 用户场景分析

### 场景1: 新用户首次使用

```bash
$ calvin deploy
# 期望: 询问 "部署到哪里？ [1] 当前项目 [2] Home目录"
# 然后保存选择到配置
```

### 场景2: 有配置的用户

```bash
# config.toml
[deploy]
target = "home"

$ calvin deploy
# 期望: 直接部署到 home，不询问
# 显示: Target: Home (~/)
```

### 场景3: 临时覆盖配置

```bash
$ calvin deploy --project  # 即使配置是 home，这次部署到 project
$ calvin deploy --home     # 即使配置是 project，这次部署到 home
```

### 场景4: Watch 命令

```bash
$ calvin watch
# 期望: 读取配置中的 target，没有配置则报错并提示
# "No deploy target configured. Run 'calvin deploy' first or set [deploy].target in config.toml"
```

---

## 推荐设计方案

### 方案: 配置优先，首次交互

#### 规则

1. **配置存在**: 使用配置中的 `[deploy].target`
2. **配置不存在 + 交互式终端**: 询问用户，保存到配置
3. **配置不存在 + 非交互式**: 报错，提示需要配置
4. **CLI flag 覆盖**: `--home` / `--project` 覆盖配置，但不修改配置

#### 配置格式

```toml
[deploy]
target = "home"      # "project" | "home"
# remote = "..."     # 可选，remote 部署目标
```

#### 命令行参数

```bash
calvin deploy           # 使用配置，无配置则询问
calvin deploy --home    # 覆盖为 home（不保存）
calvin deploy --project # 覆盖为 project（不保存）
calvin deploy --remote user@host:/path  # remote 部署

calvin watch            # 使用配置，无配置则报错
calvin watch --home     # 覆盖为 home
calvin watch --project  # 覆盖为 project
```

#### 首次运行流程

```
$ calvin deploy

No deploy target configured.

Where should Calvin deploy your prompts?

  [1] Project   - Deploy to current project (.claude/, .cursor/, etc.)
  [2] Home      - Deploy to home directory (~/.claude/, ~/.cursor/, etc.)

Your choice [1]: 2

Saved to .promptpack/config.toml:
  [deploy]
  target = "home"

🚀 Deploying to Home (~/)...
```

---

## 要删除的设计

### 删除 runtime_state.rs

- 删除 `.calvin-state.json` 机制
- 删除"记住上次选择"的逻辑
- 这是隐式状态，违反"显式优于隐式"

### 删除 config.rs 中的 DeployConfig

- 保留 `[deploy].target` 配置
- 删除复杂的默认值逻辑

---

## 用户体验对比

### Before (当前设计)

```bash
$ calvin deploy --home
✓ Deployed to home

$ calvin watch
⟳ Watching...
# 用户不知道会同步到哪里
# 如果用户忘记之前执行了 --home，会困惑
```

### After (推荐设计)

```bash
$ calvin deploy --home
✓ Deployed to home
Hint: To make this permanent, add to config.toml:
  [deploy]
  target = "home"

$ calvin watch
Error: No deploy target configured.
Run 'calvin deploy' first or add [deploy].target to config.toml
```

或者（如果有配置）：

```bash
$ calvin watch
⟳ Calvin Watch
Target: Home (~/)  # 清楚显示目标
```

---

## 实施建议

### Phase 1: 简化

1. 删除 `runtime_state.rs`
2. 删除 `.calvin-state.json` 机制
3. watch 无配置时报错

### Phase 2: 增强

1. deploy 首次运行时交互式询问
2. 询问后保存到 config.toml
3. 添加 `--project` flag 与 `--home` 对称

### Phase 3: 打磨

1. 添加 `calvin config set deploy.target home` 命令
2. deploy 成功后提示如何持久化选择

---

## 结论

**核心原则**: 用户应该始终知道系统会做什么，而不是系统"聪明地"记住用户偏好。

配置文件是唯一的真相来源。运行时状态是反模式。
