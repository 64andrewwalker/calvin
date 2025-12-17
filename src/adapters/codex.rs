//! OpenAI Codex CLI adapter
//!
//! Generates output for Codex CLI prompts:
//! - `~/.codex/prompts/<id>.md` - User-level prompts with YAML frontmatter

use std::path::PathBuf;

use crate::adapters::{Diagnostic, OutputFile, TargetAdapter};
use crate::error::CalvinResult;
use crate::models::{PromptAsset, Scope, Target};

/// Codex adapter
pub struct CodexAdapter;

impl CodexAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Generate YAML frontmatter for Codex prompts
    fn generate_frontmatter(&self, asset: &PromptAsset) -> String {
        let mut fm = String::from("---\n");
        fm.push_str(&format!("description: {}\n", asset.frontmatter.description));
        // Always include argument-hint since we always add $ARGUMENTS
        fm.push_str("argument-hint: <arguments>\n");
        fm.push_str("---\n");
        fm
    }
}

impl Default for CodexAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl TargetAdapter for CodexAdapter {
    fn target(&self) -> Target {
        Target::Codex
    }

    fn compile(&self, asset: &PromptAsset) -> CalvinResult<Vec<OutputFile>> {
        let mut outputs = Vec::new();

        // Codex prompts are typically user-level
        // User-level paths are prefixed with "~/" and resolved during sync
        // Project-level uses local .codex directory
        let base_path = if asset.frontmatter.scope == Scope::User {
            // Use HOME_DIR marker that will be resolved during sync
            // The sync engine should expand ~ to the actual home directory
            PathBuf::from("~").join(".codex").join("prompts")
        } else {
            // For project scope, use a local prompts directory
            PathBuf::from(".codex/prompts")
        };

        let path = base_path.join(format!("{}.md", asset.id));
        let frontmatter = self.generate_frontmatter(asset);
        let footer = self.footer(&asset.source_path.display().to_string());
        // Always include $ARGUMENTS after frontmatter for user input
        let full_content = format!(
            "{}\n$ARGUMENTS\n\n{}\n\n{}",
            frontmatter,
            asset.content.trim(),
            footer
        );

        outputs.push(OutputFile::new(path, full_content));

        Ok(outputs)
    }

    fn validate(&self, output: &OutputFile) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check for named placeholders without documentation
        let content = &output.content;
        
        // Find $KEY patterns (named arguments)
        let mut i = 0;
        let chars: Vec<char> = content.chars().collect();
        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() {
                let start = i + 1;
                let mut end = start;
                while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
                    end += 1;
                }
                if end > start {
                    let key: String = chars[start..end].iter().collect();
                    // Skip common placeholders
                    if !["ARGUMENTS", "1", "2", "3", "4", "5", "6", "7", "8", "9"].contains(&key.as_str()) {
                        // This is a named placeholder - should be documented
                        if !content.contains(&format!("`{}`", key)) {
                            diagnostics.push(Diagnostic {
                                severity: crate::adapters::DiagnosticSeverity::Warning,
                                message: format!(
                                    "Named placeholder ${} should be documented in usage section",
                                    key
                                ),
                                file: Some(output.path.clone()),
                            });
                        }
                    }
                }
                i = end;
            } else {
                i += 1;
            }
        }

        diagnostics
    }

    fn security_baseline(&self, _config: &crate::config::Config) -> Vec<OutputFile> {
        // Codex doesn't have project-level security config
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Frontmatter;

    #[test]
    fn test_codex_adapter_compile_user_scope() {
        let adapter = CodexAdapter::new();
        let mut fm = Frontmatter::new("Generate unit tests");
        fm.scope = Scope::User;
        let asset = PromptAsset::new(
            "gen-tests",
            "actions/gen-tests.md",
            fm,
            "Generate unit tests for the given code.",
        );

        let outputs = adapter.compile(&asset).unwrap();
        
        assert_eq!(outputs.len(), 1);
        assert_eq!(
            outputs[0].path,
            PathBuf::from("~/.codex/prompts/gen-tests.md")
        );
        // Frontmatter should be at the very beginning
        assert!(outputs[0].content.starts_with("---\n"));
        assert!(outputs[0].content.contains("description: Generate unit tests"));
        // $ARGUMENTS should be right after frontmatter
        assert!(outputs[0].content.contains("---\n\n$ARGUMENTS"));
        // Footer should be at the end
        assert!(outputs[0].content.ends_with("DO NOT EDIT. -->"));
    }

    #[test]
    fn test_codex_adapter_compile_project_scope() {
        let adapter = CodexAdapter::new();
        let mut fm = Frontmatter::new("Project-specific prompt");
        fm.scope = Scope::Project;
        let asset = PromptAsset::new(
            "project-prompt",
            "actions/project-prompt.md",
            fm,
            "Project content",
        );

        let outputs = adapter.compile(&asset).unwrap();
        
        assert_eq!(
            outputs[0].path,
            PathBuf::from(".codex/prompts/project-prompt.md")
        );
        // Frontmatter should be at the very beginning
        assert!(outputs[0].content.starts_with("---\n"));
    }

    #[test]
    fn test_codex_frontmatter() {
        let adapter = CodexAdapter::new();
        let fm = Frontmatter::new("Test prompt");
        let asset = PromptAsset::new("test", "actions/test.md", fm, "Content");

        let frontmatter = adapter.generate_frontmatter(&asset);
        
        assert!(frontmatter.starts_with("---\n"));
        assert!(frontmatter.ends_with("---\n"));
        assert!(frontmatter.contains("description: Test prompt"));
        // Always includes argument-hint since we always add $ARGUMENTS
        assert!(frontmatter.contains("argument-hint:"));
    }

    #[test]
    fn test_codex_validate_undocumented_placeholder() {
        let adapter = CodexAdapter::new();
        let output = OutputFile::new(
            "test.md",
            "Use $PROJECT_NAME for the project",
        );

        let diagnostics = adapter.validate(&output);
        
        assert!(!diagnostics.is_empty());
        assert!(diagnostics[0].message.contains("PROJECT_NAME"));
    }

    #[test]
    fn test_codex_validate_documented_placeholder() {
        let adapter = CodexAdapter::new();
        let output = OutputFile::new(
            "test.md",
            "Use $PROJECT_NAME for the project.\n\n`PROJECT_NAME` - The name of the project",
        );

        let diagnostics = adapter.validate(&output);
        
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_codex_security_baseline_empty() {
        let adapter = CodexAdapter::new();
        let config = crate::config::Config::default();
        let baseline = adapter.security_baseline(&config);
        assert!(baseline.is_empty());
    }
}
