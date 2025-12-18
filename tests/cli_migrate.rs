use std::process::Command;

#[test]
fn test_migrate_uses_box_borders() {
    let bin = env!("CARGO_BIN_EXE_calvin");

    let output = Command::new(bin)
        .arg("migrate")
        .env("TERM", "xterm-256color")
        .env("LANG", "en_US.UTF-8")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains('╭'),
        "expected migrate output to use themed box borders; got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("Already at latest version"),
        "expected migrate to explain no-op; got:\n{}",
        stdout
    );
}

