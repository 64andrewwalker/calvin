# Calvin CLI 用户体验审查报告

> **审查人**: 产品经理视角  
> **审查日期**: 2025-12-17  
> **关注重点**: 自动化决策点与用户可控性

---

## 执行摘要

Calvin 是一个 PromptOps 编译与分发工具，核心理念是"单一来源 → 多平台编译"。通过全面审查，识别出 **12 个关键自动化决策点**，其中 **5 个存在可控性问题**，需要优先改进。

### 关键发现

| 优先级 | 问题 | 影响 | 建议 |
|--------|------|------|------|
| 🔴 P0 | 安全拒绝列表强制注入 | 用户无法完全自定义 | 添加显式覆盖机制 |
| 🔴 P0 | 文件跳过时无交互确认 | 用户可能错过重要信息 | 添加交互模式 |
| 🟡 P1 | 默认启用全部目标平台 | 生成不需要的文件 | 明确提示或配置向导 |
| 🟡 P1 | MCP 白名单过于宽松 | 安全假阳性 | 支持项目级白名单 |
| 🟢 P2 | 安全模式默认 `balanced` | 可能不符合团队需求 | 首次运行时询问 |

---

## 1. 自动化决策点清单

### 1.1 安全决策

#### 🔐 D1: 强制最小拒绝列表 (Critical)

**位置**: [`src/adapters/claude_code.rs:16-24`](file:///Volumes/DevWork/projects/calvin/src/adapters/claude_code.rs#L16-L24)

**当前行为**:
```rust
const MINIMUM_DENY: &[&str] = &[
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "id_rsa",
    "id_ed25519",
    ".git/",
];
```

这些模式**始终被注入**到 `.claude/settings.json`，即使在 `yolo` 模式下也会生效。

**用户可控性**: ❌ **无法禁用**

**问题**:
- 用户无法移除特定项（如某些团队需要让 AI 读取特定 `.env.example`）
- `allow_naked = true` 配置项存在于文档但未实现

**建议**:
1. 实现 `allow_naked` 配置，允许完全自定义
2. 添加 `security.deny.exclude = [".env.example"]` 支持排除特定模式
3. 在使用 `--force` 或 `allow_naked` 时显示明确警告

---

#### 🔐 D2: MCP 服务器白名单

**位置**: [`src/security.rs:22-29`](file:///Volumes/DevWork/projects/calvin/src/security.rs#L22-L29)

**当前行为**:
```rust
const MCP_ALLOWLIST: &[&str] = &[
    "npx",
    "uvx",
    "node",
    "@anthropic/",
    "@modelcontextprotocol/",
    "mcp-server-",
];
```

**用户可控性**: ⚠️ **部分可控** (有硬编码白名单)

**问题**:
- 白名单仅包含命令前缀匹配，过于粗糙
- 配置中的 `security.mcp.allowlist` 仅在 `doctor` 检查时使用，不影响编译

**建议**:
1. 支持项目配置扩展白名单：`[security.mcp] additional_allowlist = ["my-internal-server"]`
2. 添加 `--mcp-no-validate` 选项跳过 MCP 验证
3. 在 strict 模式下要求显式声明所有 MCP 服务器

---

#### 🔐 D3: 安全模式默认值

**位置**: [`src/config.rs:28`](file:///Volumes/DevWork/projects/calvin/src/config.rs#L28)

**当前行为**: 默认 `SecurityMode::Balanced`

**用户可控性**: ✅ **可配置**

**正面评价**: 用户可通过以下方式覆盖：
- 配置文件: `[security] mode = "strict"`
- 环境变量: `CALVIN_SECURITY_MODE=yolo`
- CLI 参数: `--security-mode strict`

**改进建议**:
- 首次运行时显示安全模式选择提示
- 在 CI/CD 环境自动使用 `strict` 模式

---

### 1.2 同步决策

#### 📁 D4: 文件修改检测与跳过

**位置**: [`src/sync/mod.rs:219-236`](file:///Volumes/DevWork/projects/calvin/src/sync/mod.rs#L219-L236)

**当前行为**:
```
if current_hash != recorded_hash && !options.force {
    result.skipped.push(path_str.clone());
    continue;
}
```

**用户可控性**: ⚠️ **有限**

**问题**:
1. **无交互确认**: 用户必须在运行前决定 `--force`，无法逐个确认
2. **跳过原因不明确**: 输出仅显示 "skipped"，未说明是 hash 不匹配还是文件未追踪
3. **无合并选项**: `--merge` 在文档中提及但未实现

**建议**:
1. 添加 `--interactive`/`-i` 模式，逐个询问：
   ```
   .claude/settings.json was modified externally.
   [o]verwrite / [s]kip / [d]iff / [a]bort?
   ```
2. 区分跳过原因（用户修改 vs 未追踪文件）
3. 长期：实现结构化合并（JSON/TOML）

---

#### 📁 D5: Lockfile 自动创建

**位置**: [`src/sync/lockfile.rs`](file:///Volumes/DevWork/projects/calvin/src/sync/lockfile.rs)

**当前行为**: 首次 `sync` 自动在 `.promptpack/.calvin.lock` 创建 lockfile

**用户可控性**: ✅ **合理默认**

**正面评价**:
- Lockfile 位置明确
- 格式为人类可读的 TOML
- 可通过 `--dry-run` 预览

**改进建议**:
- 添加 `calvin lockfile reset` 命令重置追踪状态
- 支持 `calvin lockfile show` 查看当前追踪文件

---

#### 📁 D6: 原子写入策略

**位置**: [`src/sync/writer.rs`](file:///Volumes/DevWork/projects/calvin/src/sync/writer.rs)

**当前行为**: 使用 tempfile + rename 实现原子写入

**用户可控性**: ⚠️ **不可禁用**

**问题**: 某些网络文件系统（NFS、SSHFS）可能不支持原子 rename

**建议**:
- 添加 `[sync] atomic_writes = false` 配置选项（已声明但未验证实现）
- 在检测到问题时自动降级

---

### 1.3 编译决策

#### ⚙️ D7: 默认启用所有目标平台

**位置**: [`src/config.rs:206`](file:///Volumes/DevWork/projects/calvin/src/config.rs)

**当前行为**:
```toml
[targets]
enabled = ["claude-code", "cursor", "vscode", "antigravity"]
# codex 默认禁用
```

**用户可控性**: ✅ **可配置**

**问题**:
- 首次使用时用户可能只需要一个平台，但会生成所有平台的文件
- 用户可能不了解每个平台需要哪些目录

**建议**:
1. 添加 `calvin init` 命令，交互式选择目标平台
2. 在 `calvin sync` 首次运行时提示选择
3. 添加平台检测，只为已安装的 IDE 生成配置

---

#### ⚙️ D8: 策略合并 vs 拆分

**位置**: 架构设计决策 D8

**当前行为**: 默认将同一 scope 的 policies 合并为单个文件

**用户可控性**: ⚠️ **缺少显式控制**

**问题**:
- `docs/architecture.md` 提到 "默认策略必须是合并"
- 用户如何切换到拆分模式不明确

**建议**:
- 添加 `[targets.vscode] merge_mode = "split"` 配置
- 在 frontmatter 中支持 `merge: false` 强制独立生成

---

#### ⚙️ D9: 生成文件头部标记

**位置**: [`src/adapters/mod.rs`](file:///Volumes/DevWork/projects/calvin/src/adapters/mod.rs)

**当前行为**:
```markdown
<!-- Generated by Calvin. Source: .promptpack/actions/code-review.md. DO NOT EDIT. -->
```

**用户可控性**: ❌ **不可自定义**

**问题**: 某些用户可能希望自定义标记格式或禁用

**建议**:
- 添加 `[output] header_template = "..."` 配置
- 支持 `[output] include_header = false`

---

### 1.4 Doctor 检查决策

#### 🏥 D10: Doctor 检查严格程度

**位置**: [`src/security.rs:188-202`](file:///Volumes/DevWork/projects/calvin/src/security.rs#L188-L202)

**当前行为**:
| 模式 | 缺少 deny list | 未知 MCP | Turbo 模式 |
|------|---------------|---------|-----------|
| strict | ERROR | WARNING | WARNING |
| balanced | WARNING | WARNING | - |
| yolo | PASS | PASS | - |

**用户可控性**: ✅ **通过模式选择可控**

**改进建议**:
- 支持单项检查关闭：`[doctor.checks] mcp_allowlist = false`
- 添加 `--ignore` 参数忽略特定检查

---

#### 🏥 D11: Audit 退出码行为

**位置**: [`src/main.rs:655-727`](file:///Volumes/DevWork/projects/calvin/src/main.rs#L655-L727)

**当前行为**:
- Errors → exit 1
- Warnings (with `--strict-warnings`) → exit 1

**用户可控性**: ✅ **合理**

---

### 1.5 远程同步决策

#### 🌐 D12: 远程同步工具选择

**位置**: [`docs/pitfall-mitigations.md:156-178`](file:///Volumes/DevWork/projects/calvin/docs/pitfall-mitigations.md#L156-L178)

**当前行为**: 优先 rsync，Windows 降级 scp

**用户可控性**: ⚠️ **通过配置部分可控**

**建议**:
- 添加 `[remote] prefer_rsync = false` 强制使用 scp
- 显示实际使用的工具

---

## 2. 用户故事

根据以上分析，以下用户故事描述了可控性改进需求：

### 2.1 用户画像 (Personas)

根据行业标准（Cooper Personas），定义以下用户类型：

| 画像 | 描述 | 技能水平 | 核心需求 |
|------|------|----------|----------|
| 🧑‍💻 **Alex - 个人开发者** | 独立开发者，使用多个 AI 工具提效 | 中级 | 快速上手，减少配置 |
| 👩‍💻 **Sarah - 团队 Tech Lead** | 负责团队工具链标准化 | 高级 | 一致性，安全合规 |
| 🔐 **Mike - 安全工程师** | 审计 AI 工具的安全配置 | 高级 | 精确控制，审计追踪 |
| 🛠️ **Jordan - DevOps 工程师** | 维护 CI/CD 和基础设施 | 高级 | 自动化，幂等性 |
| 📝 **Taylor - 文档工程师** | 维护团队文档规范 | 中级 | 自定义输出格式 |
| 🆕 **Casey - 新手开发者** | 刚接触 AI 助手的开发者 | 初级 | 引导式体验 |

---

### 2.2 用户故事 (User Stories)

以下用户故事采用标准格式：
- **格式**: As a [persona], I want [goal] so that [benefit]
- **验收标准**: Given/When/Then (BDD style)
- **估算**: Fibonacci story points (1, 2, 3, 5, 8, 13)

---

#### Epic 1: 安全可控性 (Security Controllability)

##### US-1: 安全拒绝列表自定义

**优先级**: 🔴 P0  
**画像**: 🔐 Mike (安全工程师)

> **As a** security engineer,  
> **I want** to customize which files are added to the deny list,  
> **So that** I can protect sensitive files while allowing AI access to specific example files.

**验收标准**:
```gherkin
Given a project with `.env.example` that should be AI-accessible
When I configure `security.deny.exclude = [".env.example"]`
Then Calvin should NOT add `.env.example` to the deny list

Given a team that needs full control over security
When I set `security.allow_naked = true`
Then Calvin should display a prominent warning
And should not inject any hardcoded deny patterns

Given any security configuration change
When I run `calvin doctor`
Then the output should accurately reflect my custom rules
```

**故事点**: 3 SP  
**依赖**: 无

---

##### US-2: MCP 白名单扩展

**优先级**: 🟡 P1  
**画像**: 👩‍💻 Sarah (Tech Lead)

> **As a** tech lead,  
> **I want** to add our internal MCP servers to the allowlist,  
> **So that** doctor checks don't produce false positives for legitimate servers.

**验收标准**:
```gherkin
Given an internal MCP server "internal-code-server"
When I configure `security.mcp.additional_allowlist = ["internal-code-server"]`
Then `calvin doctor` should PASS for this server

Given the allowlist configuration
When I run `calvin doctor -v`
Then I should see which servers matched custom allowlist vs built-in
```

**故事点**: 2 SP  
**依赖**: 无

---

##### US-3: 安全模式审计追踪

**优先级**: 🟡 P1  
**画像**: 🔐 Mike (安全工程师)

> **As a** security engineer,  
> **I want** to audit who changed security mode and when,  
> **So that** I can track security posture changes over time.

**验收标准**:
```gherkin
Given security mode is changed from "strict" to "yolo"
When the change is committed
Then the lockfile should record the previous mode
And `calvin audit --history` should show the change
```

**故事点**: 3 SP  
**依赖**: US-1

---

#### Epic 2: 同步控制 (Sync Control)

##### US-4: 交互式同步确认

**优先级**: 🔴 P0  
**画像**: 🧑‍💻 Alex (个人开发者)

> **As a** developer,  
> **I want** to interactively confirm overwrites during sync,  
> **So that** I don't accidentally lose my manual edits.

**验收标准**:
```gherkin
Given I have manually edited `.claude/settings.json`
When I run `calvin sync --interactive`
Then I should be prompted with options:
  | Option      | Action                    |
  | [o]verwrite | Replace with generated    |
  | [s]kip      | Keep my version           |
  | [d]iff      | Show differences          |
  | [a]bort     | Stop sync entirely        |
  | [A]ll       | Apply choice to all files |

Given I choose "diff"
Then I should see a unified diff of changes
And be prompted again for action
```

**故事点**: 5 SP  
**依赖**: 无

---

##### US-5: Lockfile 管理

**优先级**: 🟢 P2  
**画像**: 🛠️ Jordan (DevOps)

> **As a** DevOps engineer,  
> **I want** to inspect and reset the lockfile state,  
> **So that** I can debug sync issues in CI/CD.

**验收标准**:
```gherkin
Given files are tracked in .calvin.lock
When I run `calvin lockfile show`
Then I should see a table with:
  | Path | Hash | Status | Last Synced |
  | .claude/settings.json | sha256:abc... | modified | 2025-12-17 |

When I run `calvin lockfile reset .claude/settings.json`
Then that file should be removed from tracking
And next sync should treat it as new

When I run `calvin lockfile verify`
Then I should see which tracked files have been externally modified
```

**故事点**: 2 SP  
**依赖**: 无

---

##### US-6: 结构化合并

**优先级**: 🟡 P1  
**画像**: 👩‍💻 Sarah (Tech Lead)

> **As a** tech lead,  
> **I want** Calvin to merge my manual changes with generated updates,  
> **So that** I can customize settings without losing them on sync.

**验收标准**:
```gherkin
Given I have added a custom key to `.claude/settings.json`
When I run `calvin sync --merge`
Then my custom key should be preserved
And new generated keys should be added
And conflicting keys should prompt for resolution
```

**故事点**: 8 SP  
**依赖**: US-4

---

#### Epic 3: 新手体验 (Onboarding)

##### US-7: 首次运行向导

**优先级**: 🟡 P1  
**画像**: 🆕 Casey (新手开发者)

> **As a** new user,  
> **I want** an interactive setup wizard on first run,  
> **So that** I can quickly configure Calvin for my tools.

**验收标准**:
```gherkin
Given no `.promptpack/config.toml` exists
When I run `calvin sync` for the first time
Then I should be prompted:
  "No config found. Run `calvin init` to set up? [Y/n]"

When I run `calvin init`
Then I should be asked:
  1. Which platforms do you use? (multi-select with checkboxes)
  2. Security mode? (strict/balanced/yolo with explanations)
  3. Create example prompts? [Y/n]

When I run `calvin init --yes`
Then Calvin should use sensible defaults without prompts
```

**故事点**: 3 SP  
**依赖**: 无

---

##### US-8: 平台自动检测

**优先级**: 🟢 P2  
**画像**: 🧑‍💻 Alex (个人开发者)

> **As a** developer,  
> **I want** Calvin to detect which IDEs I have installed,  
> **So that** I only get configs for tools I actually use.

**验收标准**:
```gherkin
Given I have VS Code and Cursor installed, but not Claude Code
When I run `calvin sync --auto-detect`
Then only .cursor/ and .github/ directories should be created
And .claude/ should NOT be created

Given auto-detect is enabled
When an IDE is not found
Then the output should show:
  "⏭ Skipping Claude Code (not detected)"
```

**故事点**: 5 SP  
**依赖**: US-7

---

##### US-9: 错误消息改进

**优先级**: 🟡 P1  
**画像**: 🆕 Casey (新手开发者)

> **As a** new user,  
> **I want** clear, actionable error messages,  
> **So that** I can fix problems without searching documentation.

**验收标准**:
```gherkin
Given a YAML syntax error in frontmatter
When I run `calvin sync`
Then I should see:
  "✗ Error parsing .promptpack/policies/style.md
     Line 3: Invalid YAML - unexpected ':' 
     Hint: Strings with colons need quotes: description: \"My: Rule\"
     Docs: https://calvin.dev/docs/frontmatter"

Given an unknown configuration key
When I run `calvin sync`
Then I should see:
  "⚠ Unknown config key 'securty' in .promptpack/config.toml:5
     Did you mean 'security'?"
```

**故事点**: 3 SP  
**依赖**: 无

---

#### Epic 4: 输出自定义 (Output Customization)

##### US-10: 生成文件头部自定义

**优先级**: 🟢 P2  
**画像**: 📝 Taylor (文档工程师)

> **As a** documentation engineer,  
> **I want** to customize the generated file header,  
> **So that** it matches our team's documentation standards.

**验收标准**:
```gherkin
Given I configure:
  [output]
  header_template = "<!-- Auto-generated by Calvin {{version}} from {{source}} -->"

When I run `calvin sync`
Then generated files should use my custom header format

Given I configure:
  [output]
  include_header = false

When I run `calvin sync`
Then generated files should have no header comment
```

**故事点**: 2 SP  
**依赖**: 无

---

##### US-11: 输出路径自定义

**优先级**: 🟢 P2  
**画像**: 👩‍💻 Sarah (Tech Lead)

> **As a** tech lead,  
> **I want** to customize output directory paths,  
> **So that** I can integrate Calvin with non-standard project structures.

**验收标准**:
```gherkin
Given I configure:
  [targets.claude-code]
  output_dir = ".ai/claude"

When I run `calvin sync`
Then Claude Code files should be written to .ai/claude/ instead of .claude/
```

**故事点**: 3 SP  
**依赖**: 无

---

#### Epic 5: CI/CD 集成 (CI/CD Integration)

##### US-12: 并行安全检查

**优先级**: 🟡 P1  
**画像**: 🛠️ Jordan (DevOps)

> **As a** DevOps engineer,  
> **I want** to run security audits in parallel with other CI jobs,  
> **So that** feedback is fast without slowing down the pipeline.

**验收标准**:
```gherkin
Given a CI pipeline
When I run `calvin audit --json`
Then the output should be valid JSON Lines (NDJSON)
And exit code should be 0 for pass, 1 for fail
And execution time should be < 1 second for typical projects
```

**故事点**: 2 SP  
**依赖**: 无

---

##### US-13: PR 注释集成

**优先级**: 🟢 P2  
**画像**: 🛠️ Jordan (DevOps)

> **As a** DevOps engineer,  
> **I want** Calvin to output GitHub-compatible annotations,  
> **So that** issues appear directly in PR file views.

**验收标准**:
```gherkin
Given a validation error in source file
When I run `calvin audit --format github`
Then output should use GitHub Actions annotation format:
  "::error file=.promptpack/policies/style.md,line=3::Missing required field 'description'"
```

**故事点**: 2 SP  
**依赖**: US-12

---

## 3. 可控性评分矩阵

| 决策点 | 当前可控性 | 目标可控性 | 差距 |
|--------|------------|------------|------|
| D1: 强制拒绝列表 | 0/5 | 4/5 | 🔴 4 |
| D2: MCP 白名单 | 2/5 | 4/5 | 🟡 2 |
| D3: 安全模式默认 | 5/5 | 5/5 | ✅ 0 |
| D4: 文件跳过行为 | 2/5 | 4/5 | 🔴 2 |
| D5: Lockfile 创建 | 4/5 | 5/5 | 🟢 1 |
| D6: 原子写入 | 3/5 | 4/5 | 🟢 1 |
| D7: 默认平台 | 4/5 | 5/5 | 🟢 1 |
| D8: 策略合并 | 2/5 | 4/5 | 🟡 2 |
| D9: 文件头部 | 0/5 | 3/5 | 🟡 3 |
| D10: Doctor 检查 | 4/5 | 5/5 | 🟢 1 |
| D11: Audit 退出码 | 5/5 | 5/5 | ✅ 0 |
| D12: 远程同步工具 | 3/5 | 4/5 | 🟢 1 |

**总体可控性分数**: 34/60 = **56.7%**  
**目标可控性分数**: 52/60 = **86.7%**

---

## 4. 改进路线图与进度追踪

> **状态图例**: ⬜ TODO | 🔄 In Progress | ✅ Done | ⏸️ Blocked

### Sprint 1: v0.2.0 - 关键可控性 (P0)

**目标**: 解决用户无法覆盖自动决策的核心问题  
**预计发布**: 2025-01-15

| ID | 用户故事 | 画像 | SP | 状态 | 进度 | 依赖 |
|----|---------|------|---:|:----:|-----:|------|
| US-1 | 安全拒绝列表自定义 | 🔐 Mike | 3 | ✅ | 100% | - |
| US-4 | 交互式同步确认 | 🧑‍💻 Alex | 5 | ✅ | 100% | - |

**技术任务分解**:

```
US-1: 安全拒绝列表自定义
├── [x] T1.1: 在 Config 中添加 security.deny.exclude 字段
├── [x] T1.2: 在 Config 中添加 security.allow_naked 字段  
├── [x] T1.3: 修改 ClaudeCodeAdapter 读取 exclude 配置
├── [x] T1.4: 添加 allow_naked 警告消息
├── [x] T1.5: 更新 doctor 检查反映自定义规则
└── [x] T1.6: 编写单元测试 (3+ tests)

US-4: 交互式同步确认
├── [x] T4.1: 在 SyncOptions 中添加 interactive 标志
├── [x] T4.2: 添加 CLI 参数 --interactive / -i
├── [x] T4.3: 实现交互提示 (o/s/d/a/A)
├── [x] T4.4: 实现 diff 显示功能
├── [x] T4.5: 支持 "All" 批量操作
└── [x] T4.6: 编写 CLI 测试 (2+ tests)
```

---

### Sprint 2: v0.3.0 - 核心改进 (P1)

**目标**: 提升新手体验和团队管理能力  
**预计发布**: 2025-02-15

| ID | 用户故事 | 画像 | SP | 状态 | 进度 | 依赖 |
|----|---------|------|---:|:----:|-----:|------|
| US-2 | MCP 白名单扩展 | 👩‍💻 Sarah | 2 | ✅ | 100% | - |
| US-3 | 安全模式审计追踪 | 🔐 Mike | 3 | ⬜ | 0% | US-1 |
| US-6 | 结构化合并 | 👩‍💻 Sarah | 8 | ⬜ | 0% | US-4 |
| US-7 | 首次运行向导 | 🆕 Casey | 3 | ⬜ | 0% | - |
| US-9 | 错误消息改进 | 🆕 Casey | 3 | ✅ | 100% | - |
| US-12 | 并行安全检查 | 🛠️ Jordan | 2 | ✅ | 100% | - |

---

### Sprint 3: v0.4.0 - 用户体验完善 (P2)

**目标**: 精细化控制和工具链集成  
**预计发布**: 2025-03-15

| ID | 用户故事 | 画像 | SP | 状态 | 进度 | 依赖 |
|----|---------|------|---:|:----:|-----:|------|
| US-5 | Lockfile 管理 | 🛠️ Jordan | 2 | ⬜ | 0% | - |
| US-8 | 平台自动检测 | 🧑‍💻 Alex | 5 | ⬜ | 0% | US-7 |
| US-10 | 文件头部自定义 | 📝 Taylor | 2 | ⬜ | 0% | - |
| US-11 | 输出路径自定义 | 👩‍💻 Sarah | 3 | ⬜ | 0% | - |
| US-13 | PR 注释集成 | 🛠️ Jordan | 2 | ⬜ | 0% | US-12 |

---

### 按 Epic 汇总

| Epic | 故事数 | 总 SP | 完成 | 进度 |
|------|-------:|------:|-----:|-----:|
| 🔐 安全可控性 | 3 | 8 | 2 | 62% |
| 📁 同步控制 | 3 | 15 | 1 | 33% |
| 🆕 新手体验 | 3 | 11 | 1 | 27% |
| 📝 输出自定义 | 2 | 5 | 0 | 0% |
| 🛠️ CI/CD 集成 | 2 | 4 | 1 | 50% |
| **总计** | **13** | **43** | **5** | **35%** |

---

### 按画像汇总

| 画像 | 故事数 | 优先实现 |
|------|-------:|----------|
| 🔐 Mike (安全工程师) | 2 | US-1, US-3 |
| 👩‍💻 Sarah (Tech Lead) | 3 | US-2, US-6, US-11 |
| 🧑‍💻 Alex (个人开发者) | 2 | US-4, US-8 |
| 🆕 Casey (新手开发者) | 2 | US-7, US-9 |
| 🛠️ Jordan (DevOps) | 3 | US-5, US-12, US-13 |
| 📝 Taylor (文档工程师) | 1 | US-10 |

---

## 5. 可控性评分矩阵

| 决策点 | 当前可控性 | 目标可控性 | 差距 | 相关 US |
|--------|------------|------------|------|---------|
| D1: 强制拒绝列表 | 0/5 | 4/5 | 🔴 4 | US-1 |
| D2: MCP 白名单 | 2/5 | 4/5 | 🟡 2 | US-2 |
| D3: 安全模式默认 | 5/5 | 5/5 | ✅ 0 | - |
| D4: 文件跳过行为 | 2/5 | 4/5 | 🔴 2 | US-4, US-6 |
| D5: Lockfile 创建 | 4/5 | 5/5 | 🟢 1 | US-5 |
| D6: 原子写入 | 3/5 | 4/5 | 🟢 1 | - |
| D7: 默认平台 | 4/5 | 5/5 | 🟢 1 | US-7, US-8 |
| D8: 策略合并 | 2/5 | 4/5 | 🟡 2 | US-6 |
| D9: 文件头部 | 0/5 | 3/5 | 🟡 3 | US-10 |
| D10: Doctor 检查 | 4/5 | 5/5 | 🟢 1 | US-3 |
| D11: Audit 退出码 | 5/5 | 5/5 | ✅ 0 | - |
| D12: 远程同步工具 | 3/5 | 4/5 | 🟢 1 | - |

**总体可控性分数**: 34/60 = **56.7%**  
**目标可控性分数**: 52/60 = **86.7%**

---

## 6. 结论

Calvin CLI 在**确定性**和**安全默认**方面做得很好，但在**用户可控性**方面存在改进空间。主要问题集中在：

1. **硬编码行为过多**: 某些安全机制无法被高级用户覆盖
2. **缺少交互模式**: 用户必须在运行前做所有决策
3. **黑盒操作**: 用户难以理解跳过/合并等决策的具体原因

### 优先行动项

| 优先级 | 行动 | 预期影响 |
|--------|------|----------|
| 🔴 立即 | 实现 US-1 (拒绝列表自定义) | 解除安全灵活性限制 |
| 🔴 立即 | 实现 US-4 (交互式同步) | 防止意外覆盖用户修改 |
| 🟡 短期 | 实现 US-7 (首次运行向导) | 降低新手使用门槛 |
| 🟡 短期 | 实现 US-9 (错误消息改进) | 减少用户查阅文档次数 |

### 成功指标 (建议)

| 指标 | 当前 | 目标 | 测量方式 |
|------|------|------|----------|
| 可控性分数 | 56.7% | 86.7% | 本文档评分矩阵 |
| 首次使用成功率 | 未知 | >90% | 用户反馈/telemetry |
| 配置相关 Issue 数 | 未知 | <10% | GitHub Issues |
| 文档查阅率 | 未知 | 降低 30% | 网站分析 |

---

## 附录 A: 变更日志

| 日期 | 版本 | 变更 |
|------|------|------|
| 2025-12-17 | 1.0 | 初始 UX 审查报告 |
| 2025-12-17 | 1.1 | 扩展用户画像，增加 13 个用户故事，添加 TODO 追踪 |

---

*本报告基于 Calvin v0.2.0 源码分析生成*
