//! Infrastructure Adapters
//!
//! These adapters implement the TargetAdapter port from the domain layer.
//! They transform domain Assets into platform-specific OutputFiles.

mod agents;
pub mod antigravity;
pub mod claude_code;
pub mod codex;
pub mod cursor;
pub mod opencode;
mod skills;
pub mod vscode;

pub use antigravity::AntigravityAdapter;
pub use claude_code::ClaudeCodeAdapter;
pub use codex::CodexAdapter;
pub use cursor::CursorAdapter;
pub use opencode::OpenCodeAdapter;
pub use vscode::VSCodeAdapter;

use crate::domain::ports::TargetAdapter;
use crate::domain::value_objects::Target;
use std::collections::HashMap;

/// Format extra frontmatter fields as YAML string.
///
/// Returns empty string if no extra fields, otherwise returns each field on its own line.
pub fn format_extra_frontmatter(extra: &HashMap<String, serde_yaml_ng::Value>) -> String {
    if extra.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut keys: Vec<_> = extra.keys().collect();
    keys.sort();

    for key in keys {
        if let Some(value) = extra.get(key) {
            let value_str = format_yaml_value(value);
            result.push_str(&format!("{}: {}\n", key, value_str));
        }
    }
    result
}

fn format_yaml_value(value: &serde_yaml_ng::Value) -> String {
    match value {
        serde_yaml_ng::Value::Null => "null".to_string(),
        serde_yaml_ng::Value::Bool(b) => b.to_string(),
        serde_yaml_ng::Value::Number(n) => n.to_string(),
        serde_yaml_ng::Value::String(s) => {
            if s.contains(':') || s.contains('#') || s.starts_with(' ') || s.ends_with(' ') {
                format!("\"{}\"", s.replace('\"', "\\\""))
            } else {
                s.clone()
            }
        }
        serde_yaml_ng::Value::Sequence(seq) => {
            let items: Vec<String> = seq.iter().map(format_yaml_value).collect();
            format!("[{}]", items.join(", "))
        }
        serde_yaml_ng::Value::Mapping(map) => {
            let pairs: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", format_yaml_value(k), format_yaml_value(v)))
                .collect();
            format!("{{{}}}", pairs.join(", "))
        }
        serde_yaml_ng::Value::Tagged(tagged) => format_yaml_value(&tagged.value),
    }
}

/// Get all available adapters
pub fn all_adapters() -> Vec<Box<dyn TargetAdapter>> {
    vec![
        Box::new(ClaudeCodeAdapter::new()),
        Box::new(CursorAdapter::new()),
        Box::new(VSCodeAdapter::new()),
        Box::new(AntigravityAdapter::new()),
        Box::new(CodexAdapter::new()),
        Box::new(OpenCodeAdapter::new()),
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
        Target::OpenCode => Some(Box::new(OpenCodeAdapter::new())),
        Target::All => None, // Use all_adapters() instead
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_adapters_returns_expected_count() {
        let adapters = all_adapters();
        assert_eq!(adapters.len(), 6); // All concrete adapters
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
    fn get_adapter_returns_opencode() {
        let adapter = get_adapter(Target::OpenCode);
        assert!(adapter.is_some());
        assert_eq!(adapter.unwrap().target(), Target::OpenCode);
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
