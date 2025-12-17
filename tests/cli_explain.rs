use std::process::Command;

#[test]
fn test_explain_verbose_includes_more_examples() {
    let bin = env!("CARGO_BIN_EXE_calvin");

    let output = Command::new(bin)
        .args(["explain", "-v"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);

    // These examples should only appear in verbose mode.
    assert!(
        stdout.contains("calvin deploy --targets claude-code,cursor"),
        "expected verbose examples to include target-scoped deploy; got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("calvin deploy --yes"),
        "expected verbose examples to include non-interactive deploy; got:\n{}",
        stdout
    );
}

