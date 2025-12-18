use std::path::{Path, PathBuf};

fn collect_rs_files(root: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn ui_tokens_are_not_hardcoded_outside_theme() {
    let forbidden = [
        "✓", "✗", "⚠", "↳", "⟳", "📦", "🔍", "📡", "Δ", "🟡", "🟢", "🔴", "📋", "💾", "⏳", "🔄",
        "👋", "👀", "📂", "📝", "✅",
    ];

    let mut files = Vec::new();
    collect_rs_files(Path::new("src/commands"), &mut files);
    collect_rs_files(Path::new("src/ui"), &mut files);
    collect_rs_files(Path::new("src/sync"), &mut files);

    for path in files {
        if path == Path::new("src/ui/theme.rs") {
            continue;
        }

        let content = std::fs::read_to_string(&path).expect("read source file");
        for token in forbidden {
            assert!(
                !content.contains(token),
                "{} hardcodes UI token `{}`",
                path.display(),
                token
            );
        }
    }
}
