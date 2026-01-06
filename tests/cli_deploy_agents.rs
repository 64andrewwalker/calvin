#![allow(non_snake_case)]

mod common;

use common::*;

#[test]
fn deploy_agent_writes_to_agents_directory_not_commands() {
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

    assert_deployed!(&env, ".claude/agents/reviewer.md");
    assert!(!env.project_path(".claude/commands/reviewer.md").exists());
}

#[test]
fn deploy_agent_includes_description_in_output() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/my-agent.md",
        "---
kind: agent
description: My specialized agent
scope: project
targets: [claude-code]
---
Agent content here.
",
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code"]);
    assert!(result.success);

    let content = std::fs::read_to_string(env.project_path(".claude/agents/my-agent.md")).unwrap();

    assert!(
        content.starts_with("---\n"),
        "expected YAML frontmatter:\n{content}"
    );
    assert!(
        content.contains("name: my-agent"),
        "expected name field in output:\n{content}"
    );
    assert!(
        content.contains("My specialized agent"),
        "expected description in output:\n{content}"
    );
    assert!(
        content.contains("Agent content here"),
        "expected content in output:\n{content}"
    );
}

#[test]
fn deploy_agent_cursor_generates_nothing() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/reviewer.md",
        r#"---
kind: agent
description: Code reviewer agent
scope: project
targets: [cursor]
---
You are a code reviewer.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "cursor"]);
    assert!(result.success);

    assert!(
        !env.project_path(".cursor/agents").exists(),
        "cursor should not create agents directory"
    );
    assert!(
        !env.project_path(".cursor/rules/reviewer").exists(),
        "cursor should not create rule for agent"
    );
    assert!(
        env.project_path(".cursor/commands/reviewer.md").exists(),
        "cursor should create command fallback for agent when claude-code not enabled"
    );
}

#[test]
fn deploy_agent_cursor_with_claude_code_skips_cursor_command_fallback() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/reviewer.md",
        r#"---
kind: agent
description: Code reviewer agent
scope: project
targets: [cursor, claude-code]
---
You are a code reviewer.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "cursor,claude-code"]);
    assert!(result.success);

    assert!(
        env.project_path(".claude/agents/reviewer.md").exists(),
        "claude-code should create agent file"
    );
    assert!(
        !env.project_path(".cursor/commands/reviewer.md").exists(),
        "cursor should NOT create command fallback when claude-code is enabled"
    );
}

#[test]
fn deploy_agent_user_scope_writes_to_home_agents_directory() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/global-reviewer.md",
        r#"---
kind: agent
scope: user
targets: [claude-code]
description: Global code reviewer
---
You are a global code reviewer available across all projects.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // User-scope agent should be in home directory
    let home_agent_path = env.home_path(".claude/agents/global-reviewer.md");
    assert!(
        home_agent_path.exists(),
        "user-scope agent should exist at ~/.claude/agents/global-reviewer.md"
    );

    // Verify content includes description and body
    let content = std::fs::read_to_string(&home_agent_path).unwrap();
    assert!(
        content.contains("Global code reviewer"),
        "expected description in output:\n{content}"
    );

    // User-scope agent should NOT be in project directory
    assert!(
        !env.project_path(".claude/agents/global-reviewer.md")
            .exists(),
        "user-scope agent should NOT exist in project .claude/agents/"
    );
}

#[test]
fn deploy_agent_with_empty_body_still_deploys() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/empty-agent.md",
        r#"---
kind: agent
description: An empty agent
scope: project
targets: [claude-code]
---
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert_deployed!(&env, ".claude/agents/empty-agent.md");

    let content =
        std::fs::read_to_string(env.project_path(".claude/agents/empty-agent.md")).unwrap();
    assert!(
        content.contains("An empty agent"),
        "expected description in output:\n{content}"
    );
}

#[test]
fn deploy_vscode_agent_generates_instruction_file() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/reviewer.md",
        r#"---
kind: agent
description: Code review agent
scope: project
targets: [vscode]
---
You are a code reviewer.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "vs-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let instruction_path = env.project_path(".github/instructions/reviewer.instructions.md");
    assert!(
        instruction_path.exists(),
        "vscode should create .github/instructions/reviewer.instructions.md"
    );

    let content = std::fs::read_to_string(&instruction_path).unwrap();
    assert!(
        content.contains("Code review agent"),
        "expected description in output:\n{content}"
    );
}

#[test]
fn deploy_vscode_agents_md_includes_agents_section() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/policies/code-style.md",
        r#"---
kind: policy
description: Code style guidelines
scope: project
targets: [vscode]
---
Follow these style rules.
"#,
    );

    env.write_project_file(
        ".promptpack/agents/reviewer.md",
        r#"---
kind: agent
description: Code review agent
scope: project
targets: [vscode]
---
You are a code reviewer.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "vs-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let agents_md_path = env.project_path("AGENTS.md");
    assert!(agents_md_path.exists(), "vscode should create AGENTS.md");

    let content = std::fs::read_to_string(&agents_md_path).unwrap();
    assert!(
        content.contains("## Agents"),
        "AGENTS.md should contain '## Agents' section:\n{content}"
    );
    assert!(
        content.contains("**reviewer**"),
        "AGENTS.md should contain the agent ID 'reviewer':\n{content}"
    );
    assert!(
        content.contains("Code review agent"),
        "AGENTS.md should contain the agent description:\n{content}"
    );
}

#[test]
fn deploy_agent_with_permission_mode_outputs_camelcase() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/restricted.md",
        "---
kind: agent
description: Restricted agent
scope: project
targets: [claude-code]
permission-mode: acceptEdits
---
Agent content.
",
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let content =
        std::fs::read_to_string(env.project_path(".claude/agents/restricted.md")).unwrap();

    assert!(
        content.contains("permissionMode: acceptEdits"),
        "expected permissionMode in camelCase:\n{content}"
    );
    assert!(
        !content.contains("permission-mode:"),
        "kebab-case permission-mode should NOT appear:\n{content}"
    );
}

#[test]
fn deploy_agent_with_claude_code_native_format() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/copywriter.md",
        r#"---
name: content-writer
description: Drafts marketing content
tools: Read, Grep
skills: style-guide
permissionMode: acceptEdits
---
You draft marketing content.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    let content =
        std::fs::read_to_string(env.project_path(".claude/agents/copywriter.md")).unwrap();

    assert!(
        content.contains("name: content-writer"),
        "expected name field with frontmatter value:\n{content}"
    );
    assert!(
        content.contains("tools: Read, Grep"),
        "expected tools as comma-separated:\n{content}"
    );
    assert!(
        content.contains("skills: style-guide"),
        "expected skills in output:\n{content}"
    );
    assert!(
        content.contains("permissionMode: acceptEdits"),
        "expected permissionMode in camelCase:\n{content}"
    );
}

#[test]
fn deploy_agent_infers_kind_from_directory() {
    let env = TestEnv::builder().build();

    env.write_project_file(
        ".promptpack/agents/auto-reviewer.md",
        r#"---
description: Auto reviewer (no kind specified)
scope: project
targets: [claude-code]
---
You review code automatically.
"#,
    );

    let result = env.run(&["deploy", "--yes", "--targets", "claude-code"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    assert_deployed!(&env, ".claude/agents/auto-reviewer.md");
    assert!(
        !env.project_path(".claude/commands/auto-reviewer.md")
            .exists(),
        "agent should NOT be in commands directory"
    );
}
