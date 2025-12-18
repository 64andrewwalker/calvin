use std::fs;
use std::path::Path;

fn collect_rs_files(root: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out);
                continue;
            }
            if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
}

#[test]
fn no_legacy_borders_in_commands_or_ui() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let mut files = Vec::new();
    collect_rs_files(&repo_root.join("src/commands"), &mut files);
    collect_rs_files(&repo_root.join("src/ui"), &mut files);

    // Legacy border tokens from pre-theme UI output.
    let forbidden = [
        "================================================",
        "┌─",
        "└─",
    ];

    let mut violations = Vec::new();
    for file in files {
        let Ok(content) = fs::read_to_string(&file) else {
            continue;
        };

        for token in forbidden {
            if content.contains(token) {
                violations.push(format!(
                    "{} contains forbidden token {:?}",
                    file.display(),
                    token
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Legacy border tokens must not appear in commands/ui (use theme borders / Box):\n{}",
        violations.join("\n")
    );
}
