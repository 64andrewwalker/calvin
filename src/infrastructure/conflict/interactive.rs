//! Interactive Conflict Resolver
//!
//! Prompts the user via stdin/stderr to resolve conflicts.

use crate::domain::ports::{ConflictChoice, ConflictContext, ConflictResolver};
use std::io::{self, Write};

/// Interactive conflict resolver using stdin/stderr.
///
/// Prompts the user to choose how to resolve each conflict.
/// Supports: overwrite, skip, diff, abort, and "apply to all" options.
pub struct InteractiveResolver {
    /// Track "apply to all" choice
    apply_all: std::sync::Mutex<Option<ConflictChoice>>,
}

impl InteractiveResolver {
    pub fn new() -> Self {
        Self {
            apply_all: std::sync::Mutex::new(None),
        }
    }

    fn prompt_single(&self, context: &ConflictContext) -> ConflictChoice {
        let reason_msg = match context.reason {
            crate::domain::ports::ConflictReason::Modified => "was modified externally",
            crate::domain::ports::ConflictReason::Untracked => {
                "exists but is not tracked by Calvin"
            }
        };

        loop {
            eprintln!();
            eprintln!("Conflict: {} {}", context.path.display(), reason_msg);
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
                "A" => {
                    // Ask for "apply to all" choice
                    loop {
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
                    }
                }
                _ => continue,
            }
        }
    }
}

impl Default for InteractiveResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ConflictResolver for InteractiveResolver {
    fn resolve(&self, context: &ConflictContext) -> ConflictChoice {
        // Check if "apply to all" was previously chosen
        {
            let guard = self.apply_all.lock().unwrap();
            if let Some(choice) = *guard {
                return choice;
            }
        }

        let choice = self.prompt_single(context);

        // Handle "apply to all" choices
        match choice {
            ConflictChoice::OverwriteAll => {
                let mut guard = self.apply_all.lock().unwrap();
                *guard = Some(ConflictChoice::Overwrite);
                ConflictChoice::Overwrite
            }
            ConflictChoice::SkipAll => {
                let mut guard = self.apply_all.lock().unwrap();
                *guard = Some(ConflictChoice::Skip);
                ConflictChoice::Skip
            }
            other => other,
        }
    }

    fn show_diff(&self, diff: &str) {
        eprintln!();
        eprintln!("{}", diff);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_resolver_default() {
        let resolver = InteractiveResolver::default();
        // Verify internal state is initialized correctly
        assert!(resolver.apply_all.lock().unwrap().is_none());
    }
}
