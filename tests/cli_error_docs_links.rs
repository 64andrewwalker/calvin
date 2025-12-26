//! TDD Test: Error messages should include documentation links (PRD §13.1)

mod common;

use common::env::TestEnv;

#[test]
fn no_layers_error_includes_docs_link() {
    let env = TestEnv::builder().fresh_environment().build();

    // NO .promptpack, NO user layer -> should error with "no layers found"
    let output = env.run(&["layers"]);

    assert!(!output.success);

    let stderr = output.stderr;

    // Should include a docs link
    assert!(
        stderr.contains("https://") || stderr.contains("Docs:") || stderr.contains("docs"),
        "error should include documentation link:\n{}",
        stderr
    );

    // Should include multi-layer guide
    assert!(
        stderr.contains("multi-layer")
            || stderr.contains("guides")
            || stderr.contains("64andrewwalker"),
        "error should link to multi-layer guide:\n{}",
        stderr
    );
}

#[test]
fn registry_corrupted_error_includes_docs_link() {
    let env = TestEnv::builder().fresh_environment().build();

    // Create corrupted registry
    env.write_home_file(".calvin/registry.toml", "not valid toml = = =");

    let output = env.run(&["projects"]);

    assert!(!output.success);

    let stderr = output.stderr;

    // Should include helpful information
    assert!(
        stderr.contains("registry file corrupted"),
        "should mention registry corrupted:\n{}",
        stderr
    );

    // Should include recovery instructions
    assert!(
        stderr.contains("calvin deploy") || stderr.contains("rm "),
        "should include recovery instructions:\n{}",
        stderr
    );
}
