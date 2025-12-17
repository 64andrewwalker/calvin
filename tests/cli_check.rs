use std::process::Command;

use tempfile::tempdir;

#[test]
fn test_check_includes_codex_section() {
    let dir = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_calvin");

    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["check"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Codex"),
        "check output should include Codex section; got:\n{}",
        stdout
    );
}

