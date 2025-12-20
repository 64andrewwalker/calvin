//! Target value object - defines which AI platform to compile for

use serde::{Deserialize, Serialize};

/// Target platform for compilation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Target {
    /// Claude Code (Anthropic)
    ClaudeCode,
    /// Cursor IDE
    Cursor,
    /// VS Code with GitHub Copilot
    #[serde(alias = "vscode")]
    VSCode,
    /// Google Antigravity/Gemini
    Antigravity,
    /// OpenAI Codex CLI
    Codex,
    /// All platforms (meta-target, expands to all specific targets)
    All,
}

impl Target {
    /// All concrete targets (excluding `All`)
    pub const ALL_CONCRETE: [Target; 5] = [
        Target::ClaudeCode,
        Target::Cursor,
        Target::VSCode,
        Target::Antigravity,
        Target::Codex,
    ];

    /// Returns true if this is the `All` meta-target
    pub fn is_all(&self) -> bool {
        matches!(self, Target::All)
    }

    /// Expand `All` to concrete targets, or return self if already concrete
    pub fn expand(&self) -> Vec<Target> {
        if self.is_all() {
            Self::ALL_CONCRETE.to_vec()
        } else {
            vec![*self]
        }
    }

    /// Get the directory name for this target
    pub fn directory_name(&self) -> &'static str {
        match self {
            Target::ClaudeCode => ".claude",
            Target::Cursor => ".cursor",
            Target::VSCode => ".vscode",
            Target::Antigravity => ".gemini",
            Target::Codex => ".codex",
            Target::All => ".all", // Should not be used directly
        }
    }

    /// Get a human-readable display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Target::ClaudeCode => "Claude Code",
            Target::Cursor => "Cursor",
            Target::VSCode => "VS Code",
            Target::Antigravity => "Antigravity",
            Target::Codex => "Codex",
            Target::All => "All",
        }
    }
}

impl std::fmt::Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_all_concrete_has_5_targets() {
        assert_eq!(Target::ALL_CONCRETE.len(), 5);
    }

    #[test]
    fn target_is_all() {
        assert!(Target::All.is_all());
        assert!(!Target::ClaudeCode.is_all());
    }

    #[test]
    fn target_expand_all() {
        let expanded = Target::All.expand();
        assert_eq!(expanded.len(), 5);
    }

    #[test]
    fn target_expand_concrete() {
        let expanded = Target::Cursor.expand();
        assert_eq!(expanded, vec![Target::Cursor]);
    }

    #[test]
    fn target_directory_names() {
        assert_eq!(Target::ClaudeCode.directory_name(), ".claude");
        assert_eq!(Target::Cursor.directory_name(), ".cursor");
        assert_eq!(Target::VSCode.directory_name(), ".vscode");
    }

    #[test]
    fn target_display_names() {
        assert_eq!(Target::ClaudeCode.display_name(), "Claude Code");
        assert_eq!(Target::Cursor.display_name(), "Cursor");
    }

    #[test]
    fn target_serde_kebab_case() {
        let json = "\"claude-code\"";
        let target: Target = serde_json::from_str(json).unwrap();
        assert_eq!(target, Target::ClaudeCode);
    }

    #[test]
    fn target_serde_vscode_alias() {
        let json = "\"vscode\"";
        let target: Target = serde_json::from_str(json).unwrap();
        assert_eq!(target, Target::VSCode);
    }
}
