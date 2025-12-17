use std::process::Command;

#[test]
fn test_help_mentions_interactive_mode() {
    let bin = env!("CARGO_BIN_EXE_calvin");

    let output = Command::new(bin).arg("--help").output().unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Run 'calvin' without arguments for interactive setup."),
        "help output should mention running without arguments for interactive setup; got:\n{}",
        stdout
    );
}

