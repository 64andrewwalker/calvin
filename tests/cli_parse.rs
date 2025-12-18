use std::process::Command;

use tempfile::tempdir;

#[test]
fn test_parse_avoids_legacy_box_drawing_chars() {
    let bin = env!("CARGO_BIN_EXE_calvin");

    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(promptpack.join("actions")).unwrap();
    std::fs::write(
        promptpack.join("actions/review.md"),
        "---\ndescription: Review helper\n---\nHello\n",
    )
    .unwrap();

    let output = Command::new(bin)
        .arg("parse")
        .arg("--source")
        .arg(&promptpack)
        .env("TERM", "xterm-256color")
        .env("LANG", "en_US.UTF-8")
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !stdout.contains('┌') && !stdout.contains('└'),
        "expected parse output to avoid legacy box drawing characters; got:\n{}",
        stdout
    );
    assert!(
        stdout.contains('╭'),
        "expected parse output to use themed box borders; got:\n{}",
        stdout
    );
}

