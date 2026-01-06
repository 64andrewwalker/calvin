//! Contract tests for agent deployment.
//!
//! These contracts ensure agent deployment behavior is consistent
//! across platforms and invocation methods.

use crate::common::*;

/// CONTRACT: Agents deploy to .claude/agents/ directory, not .claude/commands/
///
/// Agents are distinct from actions and should be deployed to their own
/// directory structure on platforms that support them (Claude Code).
///
/// This ensures:
/// 1. Agent files go to `.claude/agents/<id>.md` for project scope
/// 2. Agent files go to `~/.claude/agents/<id>.md` for user scope
/// 3. Agent files do NOT go to `.claude/commands/`
#[test]
fn contract_agent_deployed_to_agents_directory() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/reviewer.md",
        r#"---
kind: agent
description: Code reviewer agent
scope: project
targets: [claude-code]
---
You are a code reviewer. Review code for best practices.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let agents_path = env.project_path(".claude/agents/reviewer.md");
    let commands_path = env.project_path(".claude/commands/reviewer.md");

    assert!(
        agents_path.exists(),
        "CONTRACT: Agent must be deployed to .claude/agents/ directory"
    );
    assert!(
        !commands_path.exists(),
        "CONTRACT: Agent must NOT be deployed to .claude/commands/ directory"
    );
}

/// CONTRACT: Agent and Action with same ID produce error
///
/// Agents share the same ID namespace as actions and policies to prevent
/// collisions on platforms where they output to the same directory
/// (e.g., Cursor commands fallback, VSCode instructions).
///
/// When an agent and action have the same ID in the same layer,
/// Calvin must reject with a clear error message.
#[test]
fn contract_agent_action_same_id_conflicts() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/reviewer.md",
        r#"---
kind: agent
description: Agent reviewer
scope: project
targets: [claude-code]
---
Agent content
"#,
    );

    env.write_project_file(
        ".promptpack/actions/reviewer.md",
        r#"---
kind: action
description: Action reviewer  
scope: project
targets: [claude-code]
---
Action content
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code"]);

    assert!(
        !result.success,
        "CONTRACT: Deploy should FAIL when agent and action have same ID"
    );
    assert!(
        result.combined_output().contains("duplicate asset ID"),
        "CONTRACT: Error message should mention duplicate ID:\n{}",
        result.combined_output()
    );
    assert!(
        result.combined_output().contains("reviewer"),
        "CONTRACT: Error message should include the conflicting ID:\n{}",
        result.combined_output()
    );
}

/// CONTRACT: Cursor generates command fallback for agents when Claude Code not enabled
///
/// When deploying to Cursor without Claude Code, agents should be treated as
/// commands and deployed to .cursor/commands/ directory for usability.
#[test]
fn contract_cursor_agent_fallback_to_commands() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/helper.md",
        r#"---
kind: agent
description: Helper agent
scope: project
targets: [cursor]
---
You are a helpful agent.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "cursor"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let commands_path = env.project_path(".cursor/commands/helper.md");
    assert!(
        commands_path.exists(),
        "CONTRACT: Cursor should create command fallback for agents when Claude Code not enabled"
    );
}

/// CONTRACT: Case-insensitive IDs are merged correctly across layers
///
/// On case-insensitive filesystems (macOS, Windows), "Foo.md" and "foo.md"
/// would overwrite each other. The layer merger must treat "Foo" and "foo"
/// as the same asset ID, allowing project layer to override user layer.
#[test]
fn contract_case_insensitive_id_collision() {
    let env = TestEnv::builder()
        .with_user_layer_asset(
            "actions/Reviewer.md",
            r#"---
kind: action
description: User layer action with uppercase ID
scope: project
targets: [claude-code]
---
User layer content
"#,
        )
        .build();

    env.write_project_file(
        ".promptpack/actions/reviewer.md",
        r#"---
kind: action
description: Project layer action with lowercase ID
scope: project
targets: [claude-code]
---
Project layer content
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code"]);

    assert!(
        result.success,
        "CONTRACT: Deploy should succeed with case-insensitive merge:\n{}",
        result.combined_output()
    );

    let deployed_content = env.read_deployed_file(".claude/commands/reviewer.md");
    assert!(
        deployed_content.contains("Project layer content"),
        "CONTRACT: Project layer should override user layer for case-insensitive ID match"
    );
    assert!(
        !deployed_content.contains("User layer content"),
        "CONTRACT: User layer content should NOT appear (was overridden)"
    );
}

/// CONTRACT: Cursor skips agent command fallback when Claude Code is also enabled
///
/// When deploying to both Cursor and Claude Code, agents should only go to
/// Claude Code's .claude/agents/ directory. The Cursor command fallback
/// should NOT be created (Cursor reads from Claude's paths).
#[test]
fn contract_cursor_skips_agent_fallback_when_claude_enabled() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/helper.md",
        r#"---
kind: agent
description: Helper agent
scope: project
targets: [cursor, claude-code]
---
You are a helpful agent.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "cursor,claude-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let claude_agents_path = env.project_path(".claude/agents/helper.md");
    let cursor_commands_path = env.project_path(".cursor/commands/helper.md");

    assert!(
        claude_agents_path.exists(),
        "CONTRACT: Claude Code should create agent file in .claude/agents/"
    );
    assert!(
        !cursor_commands_path.exists(),
        "CONTRACT: Cursor should NOT create command fallback when Claude Code is enabled"
    );
}

/// CONTRACT: Agent output files must have valid YAML frontmatter
///
/// Claude Code requires agents to have YAML frontmatter with name and description.
/// Without proper frontmatter, Claude Code cannot parse agent metadata.
#[test]
fn contract_agent_output_has_yaml_frontmatter() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: Test agent
targets: [claude-code]
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "claude-code"]);

    let content = env.read_deployed_file(".claude/agents/test.md");

    assert!(
        content.starts_with("---\n"),
        "CONTRACT: Agent output MUST start with YAML frontmatter delimiter"
    );
    assert!(
        content.contains("\n---\n"),
        "CONTRACT: Agent output MUST have closing YAML delimiter"
    );
}

/// CONTRACT: Agent name field must match filesystem ID
///
/// The `name` field in agent frontmatter must equal the asset ID (filename stem).
/// This ensures consistency between Claude Code's internal routing and file paths.
#[test]
fn contract_agent_name_matches_file_id() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/my-agent.md",
        r#"---
kind: agent
description: Test agent
targets: [claude-code]
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "claude-code"]);

    let content = env.read_deployed_file(".claude/agents/my-agent.md");

    assert!(
        content.contains("name: my-agent"),
        "CONTRACT: Agent 'name' field MUST match the file ID (kebab-case)"
    );
}

/// CONTRACT: Agent description field must be present
///
/// Claude Code uses description for auto-delegation routing.
/// Empty or missing descriptions break agent discovery.
#[test]
fn contract_agent_description_present() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        r#"---
kind: agent
description: My agent description
targets: [claude-code]
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "claude-code"]);

    let content = env.read_deployed_file(".claude/agents/test.md");

    assert!(
        content.contains("description: My agent description")
            || content.contains("description: \"My agent description\""),
        "CONTRACT: Agent 'description' field MUST be present in output"
    );
}

/// CONTRACT: permission-mode transforms to permissionMode (camelCase)
///
/// Calvin source uses kebab-case (permission-mode) but Claude Code expects
/// camelCase (permissionMode). This transformation is mandatory.
#[test]
fn contract_agent_permission_mode_camelcase() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/test.md",
        "---
kind: agent
description: Test
targets: [claude-code]
permission-mode: acceptEdits
---
Content
",
    );

    env.run(&["deploy", "--yes", "--targets", "claude-code"]);

    let content = env.read_deployed_file(".claude/agents/test.md");

    assert!(
        content.contains("permissionMode: acceptEdits"),
        "CONTRACT: 'permission-mode' input MUST become 'permissionMode' (camelCase) in output\nActual content:\n{}",
        content
    );
    assert!(
        !content.contains("permission-mode:"),
        "CONTRACT: Output MUST NOT contain kebab-case 'permission-mode'"
    );
}

/// CONTRACT: Optional fields are omitted when not set
///
/// Agent output should be clean - optional fields (tools, model, permissionMode, skills)
/// should only appear when explicitly set in source.
#[test]
fn contract_agent_optional_fields_omitted() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/minimal.md",
        r#"---
kind: agent
description: Minimal agent
targets: [claude-code]
---
Content
"#,
    );

    env.run(&["deploy", "--yes", "--targets", "claude-code"]);

    let content = env.read_deployed_file(".claude/agents/minimal.md");

    assert!(
        !content.contains("tools:"),
        "CONTRACT: 'tools' field MUST be omitted when not set"
    );
    assert!(
        !content.contains("model:"),
        "CONTRACT: 'model' field MUST be omitted when not set"
    );
    assert!(
        !content.contains("permissionMode:"),
        "CONTRACT: 'permissionMode' field MUST be omitted when not set"
    );
    assert!(
        !content.contains("skills:"),
        "CONTRACT: 'skills' field MUST be omitted when not set"
    );
}
