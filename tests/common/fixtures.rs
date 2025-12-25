//! Test fixtures - reusable content constants for tests.

/// A simple policy asset for basic testing
pub const SIMPLE_POLICY: &str = r#"---
kind: policy
description: Simple test policy
scope: project
targets: [cursor]
---
# Simple Policy

This is a simple test policy.
"#;

/// A policy that targets all platforms
pub const ALL_TARGETS_POLICY: &str = r#"---
kind: policy
description: Policy for all targets
scope: project
---
# All Targets Policy

This policy applies to all supported AI coding assistants.
"#;

/// A user-scoped policy (for user layer testing)
pub const USER_POLICY: &str = r#"---
kind: policy
description: User-scoped global policy
scope: user
targets: [cursor, claude-code]
---
# User Global Policy

This is a user-level policy that applies across all projects.
"#;

/// A project-scoped policy
pub const PROJECT_POLICY: &str = r#"---
kind: policy
description: Project-specific policy
scope: project
targets: [cursor]
---
# Project Policy

This policy is specific to the current project.
"#;

/// An action asset for testing command generation
pub const SIMPLE_ACTION: &str = r#"---
kind: action
description: Test action
scope: project
---
# Test Action

Execute this action to test functionality.
"#;

/// Policy content that identifies its source layer (for merge testing)
pub const POLICY_FROM_USER: &str = r#"---
kind: policy
description: Policy from user layer
scope: project
targets: [cursor]
---
# FROM_USER

This content identifies it came from the user layer.
"#;

/// Policy content that identifies its source layer (for merge testing)
pub const POLICY_FROM_PROJECT: &str = r#"---
kind: policy
description: Policy from project layer
scope: project
targets: [cursor]
---
# FROM_PROJECT

This content identifies it came from the project layer.
"#;

/// Policy content that identifies its source layer (for merge testing)
pub const POLICY_FROM_ADDITIONAL: &str = r#"---
kind: policy
description: Policy from additional layer
scope: project
targets: [cursor]
---
# FROM_ADDITIONAL

This content identifies it came from an additional layer.
"#;

/// Minimal config enabling cursor target only
pub const CONFIG_CURSOR_ONLY: &str = r#"
[targets]
enabled = ["cursor"]
"#;

/// Config with empty targets (should disable all)
pub const CONFIG_EMPTY_TARGETS: &str = r#"
[targets]
enabled = []
"#;

/// Config enabling all common targets
pub const CONFIG_ALL_TARGETS: &str = r#"
[targets]
enabled = ["cursor", "claude-code", "vscode"]
"#;

/// Config with deploy target set to home
pub const CONFIG_DEPLOY_HOME: &str = r#"
[deploy]
target = "home"

[targets]
enabled = ["cursor"]
"#;

/// Config with deploy target set to project
pub const CONFIG_DEPLOY_PROJECT: &str = r#"
[deploy]
target = "project"

[targets]
enabled = ["cursor"]
"#;

/// User-level config with additional layers
pub const HOME_CONFIG_WITH_LAYERS: &str = r#"
[sources]
use_user_layer = true
additional_layers = []
"#;

/// Project config that ignores user layer
pub const PROJECT_CONFIG_IGNORE_USER: &str = r#"
[sources]
ignore_user_layer = true

[targets]
enabled = ["cursor"]
"#;
