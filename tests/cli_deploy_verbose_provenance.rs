//! TDD Test: Verbose deploy should show asset provenance (PRD §10.4)
//!
//! Verbosity levels:
//! - `-v`: Layer stack + asset summary + overrides
//! - `-vv`: Layer stack + full asset provenance + overrides

mod common;

use common::*;

#[test]
fn verbose_deploy_shows_layer_stack_with_asset_counts() {
    let env = TestEnv::builder()
        .with_user_asset(
            "global-rules.md",
            r#"---
kind: policy
description: Global rules
scope: project
targets: [cursor]
---
Global content
"#,
        )
        .with_project_asset(
            "local-rules.md",
            r#"---
kind: policy
description: Local rules
scope: project
targets: [cursor]
---
Local content
"#,
        )
        .build();

    let result = env.run(&["deploy", "-v", "--yes", "--targets", "cursor"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // PRD §10.4: Layer stack should show asset counts
    // Expected: "Layer Stack" or "Layers:" with "(N assets)" format
    assert!(
        result.stdout.contains("assets")
            && (result.stdout.contains("Layer") || result.stdout.contains("layer")),
        "verbose output should show layer stack with asset counts:\n{}",
        result.stdout
    );
}

#[test]
fn verbose_deploy_shows_asset_provenance() {
    let env = TestEnv::builder()
        .with_user_asset(
            "shared-style.md",
            r#"---
kind: policy
description: Shared style
scope: project
targets: [cursor]
---
Shared style content
"#,
        )
        .with_project_asset(
            "project-action.md",
            r#"---
kind: action
description: Project action
scope: project
targets: [cursor]
---
Project action content
"#,
        )
        .build();

    let result = env.run(&["deploy", "-v", "--yes", "--targets", "cursor"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // PRD §10.4: At -v level, should show asset summary
    // Full provenance list is shown at -vv
    assert!(
        result.stdout.contains("Assets:") || result.stdout.contains("merged"),
        "verbose output should show asset summary:\n{}",
        result.stdout
    );
}

#[test]
fn verbose_deploy_shows_override_info() {
    let env = TestEnv::builder()
        .with_user_asset(
            "code-style.md",
            r#"---
id: code-style
kind: policy
description: Code style (user)
scope: project
targets: [cursor]
---
User code style
"#,
        )
        .with_project_asset(
            "code-style.md",
            r#"---
id: code-style
kind: policy
description: Code style (project)
scope: project
targets: [cursor]
---
Project code style
"#,
        )
        .build();

    let result = env.run(&["deploy", "-v", "--yes", "--targets", "cursor"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // PRD §12.2: Override Confirmation should be shown
    // Should indicate that project overrides user
    assert!(
        result.stdout.contains("override") || result.stdout.contains("Override"),
        "verbose output should show override information when assets are overridden:\n{}",
        result.stdout
    );

    // Should mention the asset that was overridden
    assert!(
        result.stdout.contains("code-style"),
        "verbose output should mention the overridden asset name:\n{}",
        result.stdout
    );
}

#[test]
fn very_verbose_deploy_shows_full_provenance_list() {
    let env = TestEnv::builder()
        .with_user_asset(
            "user-rule.md",
            r#"---
kind: policy
description: User rule
scope: project
targets: [cursor]
---
User content
"#,
        )
        .with_project_asset(
            "project-rule.md",
            r#"---
kind: policy
description: Project rule
scope: project
targets: [cursor]
---
Project content
"#,
        )
        .build();

    let result = env.run(&["deploy", "-vv", "--yes", "--targets", "cursor"]);
    assert!(
        result.success,
        "deploy failed:\n{}",
        result.combined_output()
    );

    // At -vv, should show "Asset Provenance" with full list
    assert!(
        result.stdout.contains("Asset Provenance") || result.stdout.contains("Provenance"),
        "very verbose output should show Asset Provenance section:\n{}",
        result.stdout
    );

    // Should list individual assets with their source
    assert!(
        result.stdout.contains("user-rule") && result.stdout.contains("project-rule"),
        "very verbose output should list individual asset names:\n{}",
        result.stdout
    );

    // Should show source info (← notation)
    assert!(
        result.stdout.contains("←") || result.stdout.contains("<-"),
        "very verbose output should show source arrow notation:\n{}",
        result.stdout
    );
}
