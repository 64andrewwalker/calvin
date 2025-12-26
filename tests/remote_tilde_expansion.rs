//! Regression tests for remote `~` expansion.
//!
//! High-risk: when deploying to a remote destination like `host:~/projects`,
//! any remote operations that quote paths must not inhibit `~` expansion.
//!
//! This test uses a fake `ssh` binary injected via PATH to avoid network access.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use calvin::domain::ports::SyncDestination;
use calvin::infrastructure::RemoteDestination;
use tempfile::tempdir;

#[test]
fn remote_destination_expands_tilde_using_remote_home() {
    let temp = tempdir().unwrap();
    let bin_dir = temp.path().join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    let log_path = temp.path().join("ssh.log");

    // Fake `ssh`:
    // - logs args to ssh.log
    // - returns a stable fake $HOME for `echo $HOME`
    let ssh_path = bin_dir.join("ssh");
    fs::write(
        &ssh_path,
        format!(
            r#"#!/bin/sh
set -eu
echo "$@" >> "{log}"
if [ "${{2-}}" = "echo \$HOME" ]; then
  echo "/home/fake"
fi
exit 0
"#,
            log = log_path.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&ssh_path, fs::Permissions::from_mode(0o755)).unwrap();

    let original_path = std::env::var("PATH").unwrap_or_default();
    // SAFETY: single-threaded within this integration test binary.
    unsafe {
        std::env::set_var("PATH", format!("{}:{}", bin_dir.display(), original_path));
    }

    let dest = RemoteDestination::new("host:~/projects", temp.path().to_path_buf());

    // Trigger an SSH operation that needs to resolve the remote base path.
    let _ = dest.exists(Path::new(".claude/commands/test.md"));

    let log = fs::read_to_string(&log_path).unwrap_or_default();

    assert!(
        log.contains("echo $HOME"),
        "expected remote home lookup via `ssh host 'echo $HOME'`; log:\n{log}"
    );
    assert!(
        log.contains("/home/fake/projects/.claude/commands/test.md"),
        "expected `~/projects` to expand to `/home/fake/projects`; log:\n{log}"
    );
}
