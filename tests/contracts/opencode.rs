//! Contract tests for OpenCode target support.
//!
//! These contracts define the required behavior for deploying PromptPack assets
//! to OpenCode's project and user configuration directories.

use crate::common::*;

#[test]
fn contract_opencode_agent_project_path() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/reviewer.md",
        r#"---
kind: agent
description: Code reviewer agent
scope: project
targets: [opencode]
---
You are a code reviewer. Review code for best practices.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        env.project_path(".opencode/agent/reviewer.md").exists(),
        "CONTRACT: Agent MUST deploy to .opencode/agent/<id>.md"
    );
}

#[test]
fn contract_opencode_agent_user_path() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/global.md",
        r#"---
kind: agent
description: Global agent
scope: user
targets: [opencode]
---
You are a global agent.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        env.home_path(".config/opencode/agent/global.md").exists(),
        "CONTRACT: User-scope agent MUST deploy to ~/.config/opencode/agent/<id>.md"
    );
}

#[test]
fn contract_opencode_action_becomes_command() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/actions/test.md",
        r#"---
kind: action
description: Test action
scope: project
targets: [opencode]
---
Run tests for $ARGUMENTS
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        env.project_path(".opencode/command/test.md").exists(),
        "CONTRACT: Action MUST compile to .opencode/command/<id>.md"
    );

    let content = env.read_deployed_file(".opencode/command/test.md");
    assert!(
        content.contains("$ARGUMENTS"),
        "CONTRACT: $ARGUMENTS placeholder MUST be preserved"
    );
}

#[test]
fn contract_opencode_policy_compiles_to_project_agents_md() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/policies/security.md",
        r#"---
kind: policy
description: Security guidelines
scope: project
targets: [opencode]
---
Never expose secrets.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let agents_md = env.project_path("AGENTS.md");
    assert!(
        agents_md.exists(),
        "CONTRACT: OpenCode policies MUST compile into project-root AGENTS.md"
    );

    let content = env.read_deployed_file("AGENTS.md");
    assert!(
        content.contains("Security guidelines") || content.contains("security.md"),
        "CONTRACT: AGENTS.md MUST include the policy content and/or source reference"
    );
}

#[test]
fn contract_opencode_policy_user_scope_compiles_to_xdg_agents_md() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/policies/global-security.md",
        r#"---
kind: policy
description: Global security guidelines
scope: user
targets: [opencode]
---
Never expose secrets globally.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        env.home_path(".config/opencode/AGENTS.md").exists(),
        "CONTRACT: User-scope OpenCode policies MUST compile to ~/.config/opencode/AGENTS.md"
    );
}

#[test]
fn contract_opencode_skill_path_singular() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/skills/git-release/SKILL.md",
        r#"---
name: git-release
description: Create releases and changelogs
targets: [opencode]
---
# Git Release
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        env.project_path(".opencode/skill/git-release/SKILL.md")
            .exists(),
        "CONTRACT: Skill path MUST be .opencode/skill/<id>/SKILL.md (singular)"
    );
    assert!(
        !env.project_path(".opencode/skills/git-release/SKILL.md")
            .exists(),
        "CONTRACT: Skill path MUST NOT be .opencode/skills/ (plural)"
    );
}

#[test]
fn contract_opencode_skill_dedup_with_claude_code() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/skills/shared-skill/SKILL.md",
        r#"---
name: shared-skill
description: Shared skill
targets: [claude-code, opencode]
---
# Shared Skill
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code,opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert!(
        env.project_path(".claude/skills/shared-skill/SKILL.md")
            .exists(),
        "CONTRACT: When Claude Code is enabled, shared skills MUST deploy to .claude/skills/"
    );
    assert!(
        !env.project_path(".opencode/skill/shared-skill/SKILL.md").exists(),
        "CONTRACT: When Claude Code is enabled, shared skills MUST NOT duplicate under .opencode/skill/"
    );
}

#[test]
fn contract_opencode_agent_mode_default_subagent() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: Test agent
targets: [opencode]
---
Content
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let content = env.read_deployed_file(".opencode/agent/test.md");
    assert!(
        content.contains("mode: subagent"),
        "CONTRACT: OpenCode agents MUST default to mode: subagent"
    );
}

#[test]
fn contract_opencode_agent_mode_preserved() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: Test agent
targets: [opencode]
mode: primary
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/agent/test.md");
    assert!(
        content.contains("mode: primary"),
        "CONTRACT: OpenCode agent mode MUST be preserved"
    );
}

#[test]
fn contract_opencode_temperature_preserved() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: Test agent
targets: [opencode]
temperature: 0.3
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/agent/test.md");
    assert!(
        content.contains("temperature: 0.3"),
        "CONTRACT: temperature MUST be preserved in OpenCode agent output"
    );
}

#[test]
fn contract_opencode_opencode_model_overrides_model() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: Test agent
targets: [opencode]
model: sonnet
opencode-model: anthropic/claude-sonnet-4-5
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/agent/test.md");
    assert!(
        content.contains("model: anthropic/claude-sonnet-4-5"),
        "CONTRACT: opencode-model MUST override model for OpenCode output"
    );
    assert!(
        !content.contains("model: sonnet"),
        "CONTRACT: model SHOULD NOT leak into OpenCode output when opencode-model is set"
    );
}

#[test]
fn contract_opencode_command_includes_agent_and_subtask() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/actions/test.md",
        r#"---
kind: action
description: Test action
targets: [opencode]
agent: build
subtask: false
---
Run tests for $ARGUMENTS
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/command/test.md");
    assert!(
        content.contains("agent: build"),
        "CONTRACT: OpenCode command output MUST include agent when specified"
    );
    assert!(
        content.contains("subtask: false"),
        "CONTRACT: OpenCode command output MUST include subtask when specified"
    );
}

#[test]
fn contract_opencode_tools_object_format() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: Test agent
targets: [opencode]
tools:
  - Read
  - Bash
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/agent/test.md");
    assert!(
        content.contains("tools:"),
        "CONTRACT: OpenCode agent output MUST include tools when specified"
    );
    assert!(
        content.contains("read: true"),
        "CONTRACT: tools MUST map to lowercase keys (read: true)"
    );
    assert!(
        content.contains("bash: true"),
        "CONTRACT: tools MUST map to lowercase keys (bash: true)"
    );
}

#[test]
fn contract_opencode_permission_conversion_accept_edits() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: Test agent
targets: [opencode]
permission-mode: acceptEdits
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "opencode"]);

    let content = env.read_deployed_file(".opencode/agent/test.md");
    assert!(
        content.contains("permission:"),
        "CONTRACT: OpenCode agent output MUST include permission when permission-mode is set"
    );
    assert!(
        content.contains("edit: allow"),
        "CONTRACT: permission-mode acceptEdits MUST map to permission.edit: allow"
    );
}

#[test]
fn contract_opencode_warns_on_unknown_tool_names() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: Test agent
targets: [opencode]
tools:
  - Read
  - CustomTool
---
Content
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let combined = result.combined_output();
    assert!(
        combined.contains("Unknown tool 'CustomTool'"),
        "CONTRACT: Unknown tools MUST produce a warning (never fail silently).\nOutput:\n{combined}"
    );
}

#[test]
fn contract_opencode_warns_on_invalid_mode_and_defaults() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: Test agent
targets: [opencode]
mode: primry
---
Content
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "opencode"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let combined = result.combined_output();
    assert!(
        combined.contains("invalid OpenCode mode"),
        "CONTRACT: Invalid OpenCode mode MUST produce a warning.\nOutput:\n{combined}"
    );

    let content = env.read_deployed_file(".opencode/agent/test.md");
    assert!(
        content.contains("mode: subagent"),
        "CONTRACT: Invalid OpenCode mode MUST default to subagent in output"
    );
}
