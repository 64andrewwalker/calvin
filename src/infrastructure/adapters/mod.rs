//! Infrastructure Adapters
//!
//! These adapters implement the TargetAdapter port from the domain layer.
//! They transform domain Assets into platform-specific OutputFiles.

mod agents;
pub mod antigravity;
pub mod claude_code;
pub mod codex;
pub mod cursor;
mod skills;
pub mod vscode;

pub use antigravity::AntigravityAdapter;
pub use claude_code::ClaudeCodeAdapter;
pub use codex::CodexAdapter;
pub use cursor::CursorAdapter;
pub use vscode::VSCodeAdapter;

use crate::domain::ports::TargetAdapter;
use crate::domain::value_objects::Target;

/// Get all available adapters
pub fn all_adapters() -> Vec<Box<dyn TargetAdapter>> {
    vec![
        Box::new(ClaudeCodeAdapter::new()),
        Box::new(CursorAdapter::new()),
        Box::new(VSCodeAdapter::new()),
        Box::new(AntigravityAdapter::new()),
        Box::new(CodexAdapter::new()),
    ]
}

/// Get adapter for a specific target
pub fn get_adapter(target: Target) -> Option<Box<dyn TargetAdapter>> {
    match target {
        Target::ClaudeCode => Some(Box::new(ClaudeCodeAdapter::new())),
        Target::Cursor => Some(Box::new(CursorAdapter::new())),
        Target::VSCode => Some(Box::new(VSCodeAdapter::new())),
        Target::Antigravity => Some(Box::new(AntigravityAdapter::new())),
        Target::Codex => Some(Box::new(CodexAdapter::new())),
        Target::All => None, // Use all_adapters() instead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_adapters_returns_expected_count() {
        let adapters = all_adapters();
        assert_eq!(adapters.len(), 5); // All 5 adapters
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
    fn get_adapter_returns_vscode() {
        let adapter = get_adapter(Target::VSCode);
        assert!(adapter.is_some());
        assert_eq!(adapter.unwrap().target(), Target::VSCode);
    }

    #[test]
    fn get_adapter_returns_antigravity() {
        let adapter = get_adapter(Target::Antigravity);
        assert!(adapter.is_some());
        assert_eq!(adapter.unwrap().target(), Target::Antigravity);
    }

    #[test]
    fn get_adapter_returns_codex() {
        let adapter = get_adapter(Target::Codex);
        assert!(adapter.is_some());
        assert_eq!(adapter.unwrap().target(), Target::Codex);
    }

    #[test]
    fn get_adapter_all_returns_none() {
        assert!(get_adapter(Target::All).is_none());
    }

    #[test]
    fn all_adapters_covers_all_concrete_targets() {
        let adapters = all_adapters();
        let targets: Vec<Target> = adapters.iter().map(|a| a.target()).collect();

        for t in Target::ALL_CONCRETE {
            assert!(targets.contains(&t), "Missing adapter for {:?}", t);
        }
    }
}
