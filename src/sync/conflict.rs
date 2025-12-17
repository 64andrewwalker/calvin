pub(super) fn unified_diff(path: &str, old: &str, new: &str) -> String {
    use similar::TextDiff;
    TextDiff::from_lines(old, new)
        .unified_diff()
        .header(&format!("a/{}", path), &format!("b/{}", path))
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConflictReason {
    Modified,
    Untracked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ConflictChoice {
    Overwrite,
    Skip,
    Diff,
    Abort,
    OverwriteAll,
    SkipAll,
}

pub(super) trait SyncPrompter {
    fn prompt_conflict(&mut self, path: &str, reason: ConflictReason) -> ConflictChoice;
    fn show_diff(&mut self, diff: &str);
}

pub(super) struct StdioPrompter;

impl StdioPrompter {
    pub(super) fn new() -> Self {
        Self
    }
}

impl SyncPrompter for StdioPrompter {
    fn prompt_conflict(&mut self, path: &str, reason: ConflictReason) -> ConflictChoice {
        use std::io::{self, Write};

        let reason_msg = match reason {
            ConflictReason::Modified => "was modified externally",
            ConflictReason::Untracked => "exists but is not tracked by Calvin",
        };

        loop {
            eprintln!("\n⚠ {} {}", path, reason_msg);
            eprint!("[o]verwrite / [s]kip / [d]iff / [a]bort / [A]ll? ");
            let _ = io::stderr().flush();

            let mut input = String::new();
            if io::stdin().read_line(&mut input).is_err() {
                return ConflictChoice::Abort;
            }

            match input.trim() {
                "o" | "O" => return ConflictChoice::Overwrite,
                "s" | "S" => return ConflictChoice::Skip,
                "d" | "D" => return ConflictChoice::Diff,
                "a" => return ConflictChoice::Abort,
                "A" => loop {
                    eprint!("Apply to all conflicts: [o]verwrite / [s]kip / [a]bort? ");
                    let _ = io::stderr().flush();
                    let mut all = String::new();
                    if io::stdin().read_line(&mut all).is_err() {
                        return ConflictChoice::Abort;
                    }
                    match all.trim() {
                        "o" | "O" => return ConflictChoice::OverwriteAll,
                        "s" | "S" => return ConflictChoice::SkipAll,
                        "a" => return ConflictChoice::Abort,
                        _ => continue,
                    }
                },
                _ => continue,
            }
        }
    }

    fn show_diff(&mut self, diff: &str) {
        eprintln!("\n{}", diff);
    }
}

