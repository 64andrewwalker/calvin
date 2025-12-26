//! End-to-end test: lockfile version mismatch should surface a helpful error.

use std::fs;
use std::process::Command;

use tempfile::tempdir;

#[test]
fn provenance_errors_when_lockfile_version_mismatches() {
    let dir = tempdir().unwrap();
    let project_dir = dir.path();

    fs::write(
        project_dir.join("calvin.lock"),
        r#"
version = 999

[files."project:test.md"]
hash = "sha256:abc"
"#,
    )
    .unwrap();

    let bin = env!("CARGO_BIN_EXE_calvin");
    let output = Command::new(bin)
        .current_dir(project_dir)
        .args(["provenance"])
        .output()
        .unwrap();

    assert!(
        !output.status.success(),
        "expected failure, stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("lockfile format incompatible"),
        "stderr:\n{}",
        stderr
    );
    assert!(stderr.contains("calvin migrate"), "stderr:\n{}", stderr);
}
