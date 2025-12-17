你关心的 Codex 这个点，我先把结论说死：如果你想让用户在命令后面继续输入参数，然后把这些参数塞进 prompt 里，Codex 的做法就是在 prompt 文件里写占位符，然后用户在命令后面传参。最常用的是 `$ARGUMENTS`（拿到整段参数字符串）或 `$1` 到 `$9`（按位置取参数）；也支持“命名参数”那种 `KEY=value` 的形式，并在 prompt 里用 `$KEY` 引用。Codex 会在运行时做替换，如果 prompt 里出现了命名占位符而用户没传对应的 KEY，会直接报校验错误。

下面是你要的“跨平台统一维护 prompt/workflow/command，并且落地到 Claude Code / Antigravity / Codex / Cursor / VS Code / 远程 SSH 环境、覆盖 mac/win/linux”的一份可执行、尽量零歧义的 Spec。我要先把你一个隐性假设戳破：你想要“同一份 workflow 到处跑”，但这些产品对“规则/指令/命令/workflow”的语义并不一致，硬要“一份文件通吃”只会得到一堆最低公分母垃圾，最终你会回到“每个平台一套”。正确的路是：你只维护“同一个意图/同一个知识”，然后用编译器把它翻译成各平台的最佳实践形态。你要做的是 PromptOps 的“编译与分发层”，不是再发明一个 prompt 格式。

## 1. 目标与边界

本工具的目标是：在一个“源仓库”里中心化维护团队的 AI 规则（长期生效的约束）与可调用的命令/流程（显式触发的动作），然后一键生成并同步到各个平台的规范目录与文件格式，保证团队在 mac/win/linux、本机与 SSH 远程环境里得到一致体验。这里的一致不是“每个平台 UI 展示完全一样”，而是“同一个命令意图、同一套约束、同一套安全策略，在各个平台都以推荐方式生效”。

本工具明确不做三件事。第一，不做模型路由/推理层，不替代 Cursor/Claude/Copilot/Codex 的模型选择。第二，不做“偷改系统 prompt”的黑魔法，因为这类东西升级一次就炸，而且安全不可控。第三，不做 secrets 分发；任何 API key 或 token 一律通过环境变量或各平台原生安全机制注入，本工具只负责“引用”和“校验缺失”。

## 2. 总体架构

你要实现的是一个“源格式 + 多目标编译器 + 同步器”的结构。

源格式叫 PromptPack（名字随便），它放在仓库里的固定目录 `.promptpack/`。这里面只出现与你团队意图相关的内容，不出现任何平台特有字段，除非确实无法映射。然后有一组 Target Adapter：ClaudeCodeAdapter、CursorAdapter、VSCodeAdapter、AntigravityAdapter、CodexAdapter。每个 Adapter 做两件事：把源格式转成目标平台需要的文件结构；把目标平台的“最佳实践安全配置”一起生成或校验掉。

同步器负责把生成结果写到正确位置，支持三种目的地：项目内目录（随 repo 走）、用户目录（~ 下的全局配置）、远程 SSH 目录（把用户目录和项目目录都能覆盖）。同步方式必须是幂等的：同样输入，重复执行不会产生额外变更；任何生成文件都带“Generated”标记，防止你手改后又被覆盖。

## 3. 源格式 PromptPack 的目录与语义

源仓库内固定目录为 `.promptpack/`，其下只允许这四类资产，每类都用 Markdown 表达，头部用 YAML frontmatter 写元信息。

第一类是 `policies/`，代表“长期生效规则”，比如代码风格、安全约束、PR 规范、测试策略。它们会被编译到 Claude Code 的 `CLAUDE.md` 体系、Cursor 的 rules、VS Code 的 instruction files、Antigravity 的 rules。第二类是 `actions/`，代表“显式触发的命令/流程”，比如“生成单元测试”“PR review”“生成变更日志”。它们会被编译成 Claude Code 的 slash commands、Cursor 的 commands、Antigravity 的 workflows、Codex 的 prompts、VS Code 的 prompt files（如果你启用）。第三类是 `agents/`，代表“专职子代理/角色”，主要给 Claude Code 的 subagents 和 VS Code 的自定义 agent 或 AGENTS.md。Claude Code 的 subagent 文件位置是明确的（用户级 `~/.claude/agents/`、项目级 `.claude/agents/`），所以这类必须编译出目标格式。 第四类是 `mcp/`，代表需要统一分发的 MCP server 定义与环境变量引用。

每个源文件必须包含 frontmatter 的最小字段：`id`（全局唯一，kebab-case）、`title`、`kind`（policy/action/agent）、`scope`（project 或 user）、`targets`（允许/禁止在哪些平台生成）、`inputs`（可选，用来描述参数）。生成器不得接受缺失最小字段的文件，直接报错并给出行号。

为了避免你在 VS Code、Cursor 这类“多文件混入但顺序不保证”的系统里踩坑，源格式必须支持“组合编译策略”。也就是：同一个 scope 下，policy 最终可以被合并成“一个总文件”（强确定性），或者拆成多个按 apply/glob 条件加载的文件（强精细化）。默认策略必须是“合并”，只有当你明确写了 apply/glob，才拆分。

## 4. 生成结果的规范目录（平台最佳实践映射）

这里我不会给你“建议”，我给你“必须写到哪里、用什么格式、平台怎么认”。

### 4.1 Claude Code

Claude Code 侧你需要同时覆盖四个入口：记忆文件（长期规则）、自定义 slash commands（动作）、subagents（角色）、settings（权限与安全）。

自定义 slash commands 的项目级目录固定为 `.claude/commands/`，用户级目录固定为 `~/.claude/commands/`；文件名就是命令名，扩展名 `.md`，进入会话里用 `/<command>` 调用。 你的编译器必须把 `.promptpack/actions/*.md` 生成到这两个目录之一，严格按 scope 选择。命令参数在 Claude Code 里最稳的是 `$ARGUMENTS`，它会捕获用户在命令后输入的所有参数，所以你的源格式里如果声明了 `inputs`，默认应在命令模板里插入 `$ARGUMENTS` 的提示语，并在说明里写清楚参数契约（例如“第一个参数是 issue id，后面可选 priority”）。

Claude Code 的 settings 必须生成两类：可提交的 `.claude/settings.json`（团队共享默认）与不可提交的 `.claude/settings.local.json`（本机覆盖）。它的优先级链条里明确包含 `.claude/settings.local.json`、`.claude/settings.json`、`~/.claude/settings.json`，并且更具体的优先于更广泛的。 你必须利用这一点，把“安全 deny 列表”放进共享 settings，把“个人偏好”放进 local 或 user settings。

安全上最硬的一条：你必须默认生成 `permissions.deny` 来屏蔽敏感文件，例如 `.env`、`secrets/**` 等，Claude Code 会让这些文件对它“完全不可见”。 这不是可选项，这是你的工具的“强制基线”，否则你在团队推广时就是在制造数据泄漏事故。

如果你要做更高阶的“团队分发”，Claude Code 官方现在是插件系统：插件可以打包 commands、agents、skills、hooks、MCP servers，并通过 marketplace 管理。 你的工具必须支持可选的 `export claude-plugin`，把项目级输出再封装成插件结构，以便在多仓库复用，或者在企业里通过 marketplace 下发。

### 4.2 Cursor

Cursor 你要覆盖三块：rules（长期规则）、commands（动作）、MCP 配置（工具能力）。Cursor 的 rules 官方现在描述的是“每条规则是一个文件夹，里面有一个 RULE.md，RULE.md 带 frontmatter（description/globs/alwaysApply）和正文”。 这意味着你的输出目录必须长这样：

```
.cursor/rules/<rule-id>/RULE.md
```

RULE.md 的 frontmatter 里必须由你生成 `description`（从源 title/summary 来）、`globs`（从源 apply/glob 来，没有就留空或省略）、`alwaysApply`（当源 policy 标记为全局生效时 true）。Cursor 还支持不同应用方式（总是应用、智能应用、按文件模式、手动 @ 引用），而这些最终都落在 frontmatter 和 UI 类型选择上，所以你至少要生成能表达“alwaysApply”和“globs”的那部分。 另外 Cursor 文档提到 `AGENTS.md` 可以作为更简单的替代入口，这给了你一个跨平台的“低成本通道”：如果你觉得 Cursor rules 的粒度很麻烦，可以先生成 AGENTS.md，然后再逐步细分到 rules。

Cursor 的 commands 文档给出了项目内固定目录：`.cursor/commands/`，里面放 `.md` 文件，聊天里输入 `/` 会出现这些命令。 你的 `.promptpack/actions` 必须同时能生成 Cursor commands：如果 action 的目标包含 cursor，就生成一份对应的 `.md` 到 `.cursor/commands/`，并确保命令名与 Claude Code 不冲突（Cursor 命令文件名允许用更长更语义化的名字，例如 `run-all-tests-and-fix.md`）。

Cursor 的 MCP 文档强调“连接外部工具前要理解 server 做什么”，这句话虽然听起来像废话，但你应该把它变成工具级约束：你的工具默认只允许从你们的 allowlist 清单里生成 MCP servers，任何陌生 server 都需要显式批准。 MCP 配置文件位置在不同版本里会有差异，你不要赌“某一个路径永远正确”。你必须在配置里显式支持两种输出：项目级 `.cursor/mcp.json` 与用户级 `~/.cursor/mcp.json`，并在 `doctor` 子命令里做“写入后可被 Cursor 识别”的验证。你还要把一个现实写进 spec：Cursor 社区里存在“项目级 `.cursor/mcp.json` 不生效而全局生效”的实际问题案例，所以你的默认策略应该是“先全局、后项目”，并把项目级作为可选增强。

### 4.3 VS Code（含 GitHub Copilot Chat）

VS Code 的最佳实践现在非常清晰：规则用 instruction files，而不是塞在一堆 settings 里。它支持三类 Markdown 指令文件，并且会合并加入聊天上下文，但“合并的顺序不保证”。 这句话决定了你的生成策略必须以“单文件确定性”为默认，否则你迟早遇到互相覆盖、顺序变化导致行为漂移的问题。

你的输出必须支持这三条路径：

第一条是仓库级的 `.github/copilot-instructions.md`，它会自动应用到该 workspace 的所有聊天请求，并且 GitHub Copilot 在 VS Code 与 Visual Studio 等环境也会识别它。 你的工具必须默认把所有 `scope=project` 的 policy 合并生成到这个文件里，除非用户明确选择“拆分策略”。

第二条是 `.instructions.md` 文件，放在 `.github/instructions/` 目录，使用 `applyTo` frontmatter 做条件应用，适合按语言/目录拆分。 你的工具必须支持从源 policy 的 glob 映射成 `applyTo`，并在 lint 阶段检查是否出现“多个 instruction files 的 applyTo 大量重叠且内容矛盾”的情况，因为 VS Code 不承诺顺序，重叠就是不确定性。

第三条是 `AGENTS.md`。VS Code 明确支持把它作为多代理通用指令文件，并且还支持（实验性）子目录里多个 AGENTS.md。 你的工具应该把 AGENTS.md 作为跨平台的“汇总视图”，至少在仓库根生成一份，内容来源于你所有的 policy 摘要与 action 索引（不要把完整 prompt 全塞进去，否则上下文爆炸）。启用与否由 VS Code 设置控制，所以你的 `doctor vscode` 必须检查并提示用户是否打开了 `chat.useAgentsMdFile` 等设置。

还有两条必须写进 spec 的事实：VS Code 的 custom instructions 不影响你敲代码时的 inline suggestions，它只影响 chat；另外 `.instructions.md` 更偏“创建/修改文件时应用”，对纯读操作通常不应用。 这决定了你不能指望“一份规则让补全变听话”，你要把团队流程引导到 chat/agent 流，而不是 autocomplete。

### 4.4 Antigravity

Antigravity 的规则与 workflow 体系，你要按它的原生目录来生成，否则就不是最佳实践。它同时支持全局与 workspace 级，并且文件路径在官方 codelab 里写得很明确：全局规则是 `~/.gemini/GEMINI.md`，全局 workflow 是 `~/.gemini/antigravity/global_workflows/global-workflow.md`，workspace rules 在 `your-workspace/.agent/rules/`，workspace workflows 在 `your-workspace/.agent/workflows/`。 你的工具必须把 `.promptpack/policies` 编译进 `.agent/rules/`，把 `.promptpack/actions` 编译进 `.agent/workflows/`，并额外提供一个 `install --user antigravity` 把用户级规则/工作流写到 `~/.gemini/...`。

Antigravity 另一个你不能忽视的最佳实践是安全策略：它有 Terminal command auto execution 的 Off/Auto/Turbo 策略，以及 allow list/deny list 的安全模型。 你的 spec 里必须规定：默认推荐团队用“平衡模式”（类似 Auto），禁止默认 Turbo；并且你的工具要能生成或提示配置 allowlist/denylist 的路径。甚至浏览器 URL allowlist 的文件路径也在 codelab 中明确到了 `HOME/.gemini/antigravity/browserAllowlist.txt`，你要把它作为可选的安全资产纳入同步。

### 4.5 OpenAI Codex（CLI prompts）

Codex 这边你要承认一个现实：它的“prompt 资产”目前更像个人工具库，主要在用户目录。官方建议的路径是 `~/.codex/prompts/`，执行时用 `/prompts:<filename>` 触发。 你的工具必须提供 `install --user codex`，把 `.promptpack/actions` 中标记为 codex 的条目复制到 `~/.codex/prompts/`，并按 `id.md` 命名。

参数传递这块我在开头已经说死：你必须在 prompt 文件里写 `$ARGUMENTS` 或 `$1..$9` 或 `$KEY`，用户在命令后传参，Codex 会替换。 你的编译器要做两件事：第一，如果源 action 声明了 `inputs`，默认选择 `$ARGUMENTS`（因为它最通用），并在文件头自动生成一段“用法说明”；第二，如果源 action 声明了“结构化命名参数”，就生成 `$KEY` 模式，并在 lint 阶段确保 prompt 中所有命名占位符在用法里都有解释，否则这个 prompt 对用户就是陷阱，因为缺一个就会直接校验失败。

## 5. 同步与更新策略

你说的“一个地方维护，然后自动更新其他地方”，如果你指的是“改一行 prompt，所有 IDE 实时变”，那只有两种办法：第一是每个 IDE 都从同一个服务读取（MCP prompt server 之类）；第二是每个 IDE 的目录都是同一个文件的链接（symlink/hardlink）。第一种受限于平台支持程度；第二种在 Windows 和部分 Git 工作流里非常不稳。你要的是可落地、可扩展、可团队推广，所以这份 spec 规定默认采用“编译复制 + watch”的方式，链接作为可选优化。

具体要求是：工具提供 `sync`（一次性同步）、`watch`（文件变更后增量同步）、`doctor`（检测各平台是否生效）、`diff`（展示将要写入的变更）、`install --user`（写到用户目录）、`sync --remote user@host:/path`（通过 ssh/scp/rsync 复制到远端）。其中 `sync` 必须是幂等的，输出文件内容稳定，换行符按目标 OS 规范处理（Windows 输出 CRLF 可选，但默认建议统一 LF，避免 diff 噪音）。

所有生成文件必须包含不可误认的头部标记，例如：

```
<!-- Generated by calvin. Source: .promptpack/actions/code-review.md. DO NOT EDIT. -->
```

同步时如果发现目标文件不是由工具生成（缺少标记），默认不覆盖，除非用户加 `--force`。这是为了避免你把团队成员手写的规则一夜之间删干净，逼大家弃用你这套。

## 6. 远程 SSH 与多平台策略

你提到 “ssh server for these ide”，这其实是分发问题，不是 IDE 问题。你的 spec 必须规定：任何 project-scope 资产都必须存在于 repo 内（`.claude/`、`.cursor/`、`.agent/`、`.github/` 等输出目录都在仓库里），这样无论你在本机、在 VS Code Remote-SSH、在远端跑 Antigravity/Claude Code，只要打开同一个 repo，规则和命令都天然随代码走。

用户级（user-scope）资产才需要同步到远端 home 目录。你的工具必须能在远端执行两种策略：第一种是在远端安装同一个 promptpack 仓库，然后 `calvin install --user`；第二种是本机 `calvin sync --remote` 直接推送到远端指定 home 下的 `.claude/`、`.gemini/`、`.codex/`、`.cursor/` 等目录。两种策略你都要支持，因为企业里经常不允许把你们内部 prompt 仓库 clone 到生产跳板机。

## 7. 安全基线（你现在很可能低估了）

我直说：你们一旦把“可执行命令的 agent”推广到团队，又没有强制 deny/allow 策略，你们不是在提效，是在扩大攻击面。Claude Code 这边必须默认生成 `permissions.deny` 去屏蔽 secrets 文件，否则 Claude 会在你不注意时读到 `.env`，这不是概率问题，是迟早发生的问题。 Antigravity 这边必须避免默认 Turbo，并且把 allowlist/denylist、browser allowlist 纳入治理，否则 prompt injection 这类事会从“安全文章”变成“你们的事故复盘”。 Cursor 这边对 MCP server 必须默认白名单，因为 MCP 等于把外部能力注入 agent。

因此 spec 强制要求工具内置 `security baseline` 模块：它会在生成时自动附加一份“平台安全配置建议/模板”，并在 `doctor` 里检查是否缺失（例如 `.claude/settings.json` 没有 deny 规则就报错；Antigravity 发现是 Turbo 就给红色警告；Cursor 发现 MCP server 来自非白名单就阻止写入）。

## 8. 技术选型（交叉编译与维护成本）

你要的不是“能跑”，而是“能被团队长期用”。技术栈我建议你别纠结花活，选 Rust 或 Go 都行，但你必须把以下硬要求写进 spec：单一静态可执行文件；mac/win/linux 三端交叉编译；不依赖 Python/Node 运行时；内置文件 watcher；内置 YAML frontmatter 解析；内置模板渲染（最简单的变量替换即可）；内置 JSON/TOML 配置解析；提供 `--json` 输出给 CI 使用。

如果你们内部有强 Node 文化，也可以用 Node + pkg/deno compile，但你会在企业 Windows 环境、杀软、路径权限上付更多隐性成本。你要的是“部署阻力最小”，不是“写起来最爽”。

## 9. 交付顺序（你该先做什么，不要再拖）

第一优先级，你先做“编译到项目内目录”这条链路，只覆盖 Claude Code、Cursor、VS Code，因为它们的目录规则清晰、团队接受度高，而且都能直接放进 repo 里。Claude Code 的命令目录 `.claude/commands/` 与 Cursor 的 `.cursor/commands/`、VS Code 的 `.github/copilot-instructions.md` 一旦生成出来，团队当天就能用起来。

第二优先级，你做 Antigravity 的 `.agent/rules/` 和 `.agent/workflows/`，因为它也支持把这些放进 workspace，并且路径明确。

第三优先级，你做 user-scope 的安装器（`~/.claude/commands/`、`~/.codex/prompts/`、`~/.gemini/...` 等），以及远程 sync。Codex 放在第三是因为它偏个人工具库，而且你要先把“源格式的参数契约”打磨好，否则会变成一堆不可用的 prompt。

如果你想，我可以在你确定源格式字段后，把上面这份 spec 进一步“落到文件级别”：给出 `.promptpack/` 的完整示例树、每类文件的 frontmatter 约束、以及每个 Adapter 的输入输出契约（包括错误码、日志格式、doctor 检查项）。你现在要做的关键决策只有一个：你们愿不愿意把“规则治理”当成一等工程资产，而不是一堆散落在各个 IDE 里的 prompt 小作文。只要你们不愿意治理，这个工具再漂亮也会沦为摆设。
