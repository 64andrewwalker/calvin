# 你要兼容的“Claude agent 规范”到底指什么

在 Claude 生态里，“agent”这个词会被用在不同层级上，但你现在说的“claude agent 规范”，结合你提到的 `.claude/`、skills、slash commands、以及希望多个 agent 协作完成工作，我建议你把目标锁定为 Claude Code 的“Subagents（子代理）”规范：也就是用一个带 YAML frontmatter 的 Markdown 文件来定义一个可被委派的专家角色，它有独立的上下文窗口、可配置工具权限、可选模型与权限模式，并且可以被主线程自动或显式地调用。

你要实现的“兼容”，本质上就是让你在 OpenCode 里定义的 agent 也遵循 Claude Code Subagents 的文件结构与字段语义，这样同一套 `.claude/agents`、`.claude/skills`、`.claude/commands` 未来既能在 Claude Code 里跑，也能通过 OpenCode/oh-my-opencode 跑起来。

# Claude Code Subagent 的文件规范

Claude Code 的自定义 subagent 是“一个 Markdown 文件 + YAML frontmatter + system prompt 正文”。它放在项目级或用户级目录中；项目级优先级更高。

你需要兼容的核心结构长这样（这是规范形状，不是唯一写法）：

```md
---
name: code-reviewer
description: Review code changes proactively after edits; focus on security, performance, maintainability.
tools: Read, Grep, Glob  # 可选；不写就继承主线程工具
model: sonnet            # 可选：sonnet/opus/haiku 或 inherit
permissionMode: default  # 可选：default/acceptEdits/dontAsk/bypassPermissions/plan/ignore
skills: pr-review, security-check  # 可选：启动时预加载的 Skills
---
你是一个资深代码审查专家。你的目标是以“可执行的改进建议”为输出，指出问题、风险等级、最小修复方案与验证办法。不要直接修改文件；除非用户明确要求你给出 patch。
```

这些字段的意义必须对齐 Claude Code：`name` 与 `description` 是必填；`tools` 是可选的“白名单工具列表（逗号分隔）”，不写就继承主线程工具；`model` 可以写 `sonnet/opus/haiku` 或 `inherit`；`permissionMode` 控制子代理在权限提示上的行为；`skills` 是可选的“启动时预加载 skills 列表”。

这里有两个非常关键、很容易被忽略但会决定“多 agent 协作是否好用”的点。第一，Claude Code 会依据你在用户请求里描述的任务、以及各 subagent 的 `description` 字段来做自动委派，所以 `description` 不是“介绍”，而是“路由规则”；官方甚至建议在 `description` 里使用类似 “use PROACTIVELY / MUST BE USED” 的措辞来提高被自动调用的概率。 第二，subagent 与主线程的上下文是隔离的，这能防止主对话被大量探索细节污染，但也意味着你要显式设计“子代理输出给主线程的交付物格式”，否则主线程会拿不到可以直接执行的结果。

# Claude Code 的工具集合与权限模型

Claude Code 环境里（也就是 Claude Code/Claude Agent SDK 这一套工具体系），“agent 能干什么”不是靠你 prompt 里说了算，而是由可用工具（tools）和权限系统共同决定。官方在设置文档里列出了 Claude Code 可用的内部工具，包括读写文件、执行 Bash、WebFetch/WebSearch、Skill、SlashCommand、以及用于调用 subagent 的 Task 等。

你在 `.claude/agents/*.md` 里写的 `tools` 字段，就是对这些工具的显式选择；如果你不写 `tools`，该 subagent 默认继承主线程的全部工具（包括 MCP 工具）。 另外，`permissionMode` 有一组固定取值（`default`, `acceptEdits`, `dontAsk`, `bypassPermissions`, `plan`, `ignore`），它决定子代理如何处理权限请求。

如果你准备把这套 agent 放进团队仓库长期使用，建议你把“安全边界”下沉到 `.claude/settings.json` 的权限规则里，而不是只靠 agent prompt 自律。Claude Code 明确支持在 `permissions.deny` 里把 `.env`、`secrets/**` 等敏感文件完全从 Claude 视野里移除，从根上避免泄露。

# Skill：让 agent“学会做事”的规范，以及 subagent 如何用 Skill

Skill 在 Claude Code 里是“模型自动触发”的能力扩展：启动时只加载每个 Skill 的 name/description 元信息；当用户请求和 description 匹配时，Claude 会请求启用该 Skill，然后把完整 `SKILL.md` 内容加载进上下文。 Skill 的目录位置同样分项目级与用户级，常见是 `.claude/skills/<skill-name>/SKILL.md` 或 `~/.claude/skills/<skill-name>/SKILL.md`。

`SKILL.md` 的规范重点是 YAML frontmatter。Claude Code 文档明确：必须包含 `name` 与 `description`；可选字段包含 `allowed-tools`（Skill 激活时允许的工具集合）与 `model`（Skill 激活时使用的模型）。 同时官方建议用“渐进披露”的方式把大段参考材料拆分到同目录下的其他文件，通过链接让 Claude 在需要时再读，避免一上来占满上下文；并且建议把 `SKILL.md` 控制在 500 行以内。

你特别关心“agent 能调用我们的 skill”。在 Claude Code 里有一个容易踩坑的规则：自定义 subagent 默认不会继承主线程的 Skills，要让 subagent 具备 Skills，你必须在 subagent 文件的 frontmatter 里显式写 `skills:` 列表来预加载；并且官方还说明内置的 Explore/Plan/Verify 以及 Task tool 本身都不会访问你的 Skills，只有你自己定义的 custom subagents 才能通过 `skills` 字段拿到 Skills。

在 OpenCode 这边，skills 也以 “SKILL.md + 目录” 形式被发现，并且 OpenCode 明确兼容 `.claude/skills/<name>/SKILL.md` 这种路径；同时它提供了原生的 `skill` 工具用于按需加载某个 skill 的完整内容，调用形式类似 `skill({ name: "git-release" })`。 这对你很关键，因为你现在已经在用 skills 编排工作，把 skills 作为“组织知识与流程的最小单元”是正确方向；你要做的是在多 agent 时代，把不同 skill 分配给不同 subagent（通过 `skills:` 预加载或工具权限控制），让每个 subagent 在自己的上下文里按需取用。

# Slash command：规范、可调用性、以及如何让 agent 触发 command

Claude Code 的自定义 slash command 是“一个 Markdown 文件”，放在 `.claude/commands/`（项目级）或 `~/.claude/commands/`（用户级），文件名（不含 `.md`）就是命令名。 它支持用子目录做 namespacing；当项目级与用户级同名时，项目级覆盖用户级。

你说“agent 能调用我们的 command”，在 Claude Code 里这是有明确机制的：有一个 `SlashCommand` 工具允许 Claude 在对话中“程序化执行自定义 slash commands”。官方建议如果你希望 Claude 主动用它，就在 prompt 或 `CLAUDE.md` 里直接点名（带 `/`）命令，例如“在你要开始写测试时运行 /write-unit-test”。

但并不是所有命令都能被 `SlashCommand` 工具调用。官方明确限制：它只支持用户自定义命令（不是 `/init`、`/compact` 这种内置命令），并且要求该命令的 frontmatter 里必须有 `description` 字段，因为描述会被放进上下文供模型选择。 你还可以用 `disable-model-invocation: true` 在单个命令上禁止模型通过工具调用它（这同样会把该命令的元信息从上下文移除），非常适合把“危险但偶尔需要”的命令保留给人工显式触发。

另外，当你项目里命令很多时，Claude Code 会对 `SlashCommand` 工具注入的命令元信息设字符预算（默认 15000 字符），预算可通过 `SLASH_COMMAND_TOOL_CHAR_BUDGET` 环境变量调整。 这意味着你要写“短而信息密度高”的 command description，避免命令太多导致重要命令的描述被挤掉。

# 在 OpenCode + oh-my-opencode 中落地“Claude 规范 subagents”

OpenCode 自己也支持 agents，并且既支持在 `opencode.json` 里配置，也支持用 Markdown 文件定义，存放在 `.opencode/agent/` 或 `~/.config/opencode/agent/`，文件名就是 agent 名。 它的 agent frontmatter 字段与 Claude Code 不完全一样（例如它有 `mode: subagent/primary/all`、以及 `tools:` 的布尔开关等），所以如果你要“兼容 Claude agent 规范”，建议你把“权威来源”定为 Claude Code 的 `.claude/agents/*.md`，然后让 OpenCode 侧通过 oh-my-opencode 的 Claude Code compatibility 去加载/适配。

你提到你已经在用 oh-my-opencode 来调用 agent。oh-my-opencode 的 Agent System 文档里明确提供了 agent 调用工具（例如 `call_omo_agent` 与 `call_omo_agent_background`）来触发某个 agent 执行任务，并且还描述了它对“后台任务”和“上下文隔离”的支持思路。 这和 Claude Code 的 Task/subagent 模型在“组织形态”上是同构的：主线程做编排，专用 subagent 做分工，最后把结构化结果回传给主线程合并执行。你要做的是把你的 skills/commands 当作“可复用动作库”，再把 subagents 当作“会选动作库并能独立思考/产出交付物的专家”。

# 一个可直接照抄的“专家级多 agent 协作”落地方案

下面是一套我推荐你从零启动时就按“可扩展、多 agent、可控权限、可迁移到 Claude Code”去设计的最小系统。它不要求你立刻把一切做复杂，而是先把接口、边界、与可组合性立起来。

首先，把目录结构统一到 `.claude/`（这就是你“兼容 Claude 规范”的根基）：

```text
.claude/
  agents/
    implementer.md
    reviewer.md
    researcher.md
  skills/
    repo-navigation/
      SKILL.md
    engineering-standards/
      SKILL.md
  commands/
    kickoff.md
    verify.md
  settings.json
```

然后你定义三个 subagent：researcher 负责探索与收集证据，implementer 负责产出可执行改动方案与补丁（在 OpenCode 里通常会用写文件工具），reviewer 负责质量与安全审计。它们都用明确的交付物格式回传，让主线程能稳定合并。

下面是一个 researcher 模板（重点是：它的工具尽量只读，它的输出必须“引用文件路径与证据”，避免主线程再重复探索）：

```md
---
name: researcher
description: Use proactively to explore the codebase and gather evidence (file paths, snippets, commands) before planning or editing.
tools: Read, Glob, Grep, Bash
model: haiku
permissionMode: default
skills: repo-navigation
---
你是代码库研究员。你只做“发现与证据整理”，不做改动。你的输出必须包含：你查看过的文件路径、你使用过的搜索关键词或命令、以及你认为关键的片段或结论为何成立。你要尽量减少无关信息，确保主线程可以直接据此做计划与改动。
```

下面是 implementer 模板（重点是：它预加载工程规范类 skill；它要把改动拆到最小、可回滚；它必须告诉主线程如何验证；如果你在 OpenCode 里允许它写文件，你再通过权限规则限制范围）：

```md
---
name: implementer
description: Implement changes safely with minimal diff; always include verification steps and rollback strategy.
tools: Read, Glob, Grep, Edit, Write, Bash, Skill, SlashCommand
model: sonnet
permissionMode: acceptEdits
skills: engineering-standards, repo-navigation
---
你是实施工程师。你要把任务转化为“最小可行改动”，并且始终给出验证步骤。你在执行前先说明你将改哪些文件、为何改、可能风险点与回滚方式。你在需要时可以加载 Skills，并在合适的时机调用自定义 slash commands（例如 /verify）来执行标准化流程。除非用户明确要求，否则不要做大规模重构。
```

下面是 reviewer 模板（重点是：禁止写；输出要能被主线程直接转成待办或修复清单）：

```md
---
name: reviewer
description: Review changes proactively after edits; focus on security, correctness, performance, and maintainability. Provide actionable fixes.
tools: Read, Glob, Grep, Bash, Skill
model: sonnet
permissionMode: default
skills: engineering-standards
---
你是审查员。你只输出“问题 -> 影响 -> 修复建议 -> 如何验证”。如果你认为存在安全风险或数据一致性风险，必须明确标注严重性，并给出最小修复方案与替代方案。不要直接改文件。
```

接着，你把技能拆成两类：一类是“仓库导航/上下文”（比如项目目录约定、关键模块入口、常用命令），另一类是“工程标准”（比如你团队的代码风格、提交规范、测试策略）。Skill 的 frontmatter 必须有 `name` 与 `description`，并且 description 要尽量贴近你平时会怎么提需求，因为 Claude 是靠它触发的。 你如果希望某个 skill 激活时严格只读，可以用 `allowed-tools: Read, Grep, Glob`。

最后，你定义两个 commands：一个 `/kickoff` 用来把用户需求“转成多 agent 流程”，一个 `/verify` 用来做统一验证。关键是：如果你希望 agent 能通过 `SlashCommand` 工具调用它们，那么它们必须有 `description`，且不要设置 `disable-model-invocation: true`。

`kickoff.md` 可以这样写（示例强调的是“把编排固化在 command 里”，让你不需要每次手工指挥）：

```md
---
description: Start a multi-agent workflow: research -> plan -> implement -> review -> verify.
---
请按照如下协作方式完成任务：先调用 researcher 收集证据与代码入口，然后基于证据给出计划；计划通过后调用 implementer 产出最小改动并说明验证步骤；最后调用 reviewer 审查风险与改进点；如果存在 /verify 命令，请在收尾阶段运行它并汇总输出。
```

你还需要把 `.claude/settings.json` 补上基本的安全策略，至少把 `.env`、`secrets/**` 这种路径 deny 掉，避免 agent 意外读到敏感信息。

# 你如何从“能跑”迭代到“专家级”

当你把上面跑通后，真正把它进化成“专家级”的关键，不是再加更多 agent，而是把每个 agent 的输入输出契约固化下来，让它们在复杂项目里仍然可组合、可预测。你可以把 reviewer 的输出格式、implementer 的验证清单格式、researcher 的证据格式，都写进 `engineering-standards` skill 里，并且让三个 subagent 都预加载该 skill；这样“流程约束”不是散落在 prompt 里，而是集中管理、可版本化、可被团队复用。

当你想让 agent 主动触发某些命令时，优先用 `SlashCommand` 工具的推荐做法：在 agent prompt 或 `CLAUDE.md` 里直接点名“在什么时机运行 /某命令”，并确保命令带 `description`。 同时，把危险命令用 `disable-model-invocation: true` 关掉模型调用入口，只允许你手工敲；这是“可自治”和“可控风险”之间非常实用的折中。

在 OpenCode 侧，如果你发现某个 subagent 权限太大或太小，不要只改 prompt，直接在 agent 或全局配置里收紧 tools/permissions。OpenCode 的 agent 定义支持在 frontmatter 里配置权限与工具开关，甚至可以对 bash 命令做模式匹配授权；这非常适合把 implementer 的 bash 限制在 `npm test`、`pnpm lint`、`git diff` 这类白名单上。

如果你愿意，我也可以基于你现在已经存在的 skills 与 slash commands（你把目录结构或其中一两个文件内容贴出来即可），把它们重新分层成“工程标准 skill + 若干流程 command + 三到五个 subagents”，并给你一套直接可复制到 `.claude/` 的最终版本，让你在 OpenCode 里马上进入多 agent 协作状态。
