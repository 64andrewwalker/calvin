//! Infrastructure Adapters
//!
//! These adapters implement the TargetAdapter port from the domain layer.
//! They transform domain Assets into platform-specific OutputFiles.

pub mod claude_code;
pub mod cursor;

pub use claude_code::ClaudeCodeAdapter;
pub use cursor::CursorAdapter;

use crate::domain::ports::TargetAdapter;
use crate::domain::value_objects::Target;

/// Get all available adapters
pub fn all_adapters() -> Vec<Box<dyn TargetAdapter>> {
    vec![
        Box::new(ClaudeCodeAdapter::new()),
        Box::new(CursorAdapter::new()),
        // TODO: Add VSCode, Antigravity, Codex adapters
    ]
}

/// Get adapter for a specific target
pub fn get_adapter(target: Target) -> Option<Box<dyn TargetAdapter>> {
    match target {
        Target::ClaudeCode => Some(Box::new(ClaudeCodeAdapter::new())),
        Target::Cursor => Some(Box::new(CursorAdapter::new())),
        // TODO: Add VSCode, Antigravity, Codex
        Target::VSCode | Target::Antigravity | Target::Codex => None,
        Target::All => None, // Use all_adapters() instead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_adapters_returns_expected_count() {
        let adapters = all_adapters();
        // Currently only Claude Code and Cursor are migrated
        assert_eq!(adapters.len(), 2);
    }

    #[test]
    fn get_adapter_returns_claude_code() {
        let adapter = get_adapter(Target::ClaudeCode);
        assert!(adapter.is_some());
        assert_eq!(adapter.unwrap().target(), Target::ClaudeCode);
    }

    #[test]
    fn get_adapter_returns_cursor() {
        let adapter = get_adapter(Target::Cursor);
        assert!(adapter.is_some());
        assert_eq!(adapter.unwrap().target(), Target::Cursor);
    }

    #[test]
    fn get_adapter_all_returns_none() {
        assert!(get_adapter(Target::All).is_none());
    }
}
