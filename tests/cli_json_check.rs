use std::process::Command;

use serde_json::Value;
use tempfile::tempdir;

#[test]
fn test_check_json_emits_ndjson_event_stream() {
    let dir = tempdir().unwrap();
    let bin = env!("CARGO_BIN_EXE_calvin");

    let output = Command::new(bin)
        .current_dir(dir.path())
        .args(["check", "--json"])
        .output()
        .unwrap();

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(
        lines.len() > 1,
        "expected NDJSON (multiple lines), got:\n{stdout}"
    );

    let first: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(first["event"], "start");
    assert_eq!(first["command"], "check");

    let last: Value = serde_json::from_str(lines[lines.len() - 1]).unwrap();
    assert_eq!(last["event"], "complete");
    assert_eq!(last["command"], "check");

    assert!(
        lines.iter().any(|l| {
            serde_json::from_str::<Value>(l)
                .ok()
                .is_some_and(|v| v["event"] == "check")
        }),
        "expected at least one check event, got:\n{stdout}"
    );
}
