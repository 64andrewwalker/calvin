//! Conflict resolution for sync operations
//!
//! Provides `ConflictResolver` trait for handling file conflicts during sync.
//! Use `InteractiveResolver` for production (stdin/stderr prompts) or
//! implement a mock resolver for testing.

pub fn unified_diff(path: &str, old: &str, new: &str) -> String {
    use similar::TextDiff;
    TextDiff::from_lines(old, new)
        .unified_diff()
        .header(&format!("a/{}", path), &format!("b/{}", path))
        .to_string()
}

/// Reason why a file is in conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictReason {
    /// File was modified externally after it was deployed
    Modified,
    /// File exists but is not tracked by Calvin
    Untracked,
}

/// User's choice for resolving a conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictChoice {
    /// Overwrite the file with new content
    Overwrite,
    /// Skip this file, keep existing content
    Skip,
    /// Show diff between existing and new content
    Diff,
    /// Abort the entire sync operation
    Abort,
    /// Overwrite this and all remaining conflicts
    OverwriteAll,
    /// Skip this and all remaining conflicts
    SkipAll,
}

/// Trait for resolving conflicts during sync
///
/// Implement this trait to customize conflict resolution behavior.
/// Use `InteractiveResolver` for production (prompts user via stdin/stderr).
pub trait ConflictResolver {
    /// Prompt to resolve a single conflict
    fn resolve_conflict(&mut self, path: &str, reason: ConflictReason) -> ConflictChoice;
    /// Display a diff to the user
    fn show_diff(&mut self, diff: &str);
}

/// Interactive conflict resolver using stdin/stderr
pub struct InteractiveResolver;

impl InteractiveResolver {
    pub fn new() -> Self {
        Self
    }
}

impl Default for InteractiveResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictResolver for InteractiveResolver {
    fn resolve_conflict(&mut self, path: &str, reason: ConflictReason) -> ConflictChoice {
        use std::io::{self, Write};

        let reason_msg = match reason {
            ConflictReason::Modified => "was modified externally",
            ConflictReason::Untracked => "exists but is not tracked by Calvin",
        };

        loop {
            eprintln!("\nConflict: {} {}", path, reason_msg);
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
