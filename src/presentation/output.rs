//! Output Rendering
//!
//! Provides a unified interface for rendering output to different formats.

use std::path::Path;

use crate::application::DeployResult;

/// Output format for rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// Human-readable text output
    #[default]
    Text,
    /// JSON output for scripting
    Json,
    /// Minimal output (CI-friendly)
    Minimal,
}

/// Icons for output rendering
struct Icons {
    check: &'static str,
    cross: &'static str,
    write: &'static str,
    skip: &'static str,
    trash: &'static str,
}

impl Icons {
    fn unicode() -> Self {
        Self {
            check: "✓",
            cross: "✗",
            write: "→",
            skip: "○",
            trash: "🗑",
        }
    }

    fn ascii() -> Self {
        Self {
            check: "[OK]",
            cross: "[FAIL]",
            write: "->",
            skip: "[ ]",
            trash: "[DEL]",
        }
    }
}

/// Trait for rendering deploy results
pub trait DeployResultRenderer {
    /// Render the deploy result
    fn render(&self, result: &DeployResult, source: &Path);
}

/// Text renderer for deploy results
pub struct TextRenderer {
    /// Whether to use colors
    pub color: bool,
    /// Whether to use unicode
    pub unicode: bool,
    /// Verbosity level
    pub verbose: u8,
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self {
            color: true,
            unicode: true,
            verbose: 0,
        }
    }
}

impl DeployResultRenderer for TextRenderer {
    fn render(&self, result: &DeployResult, source: &Path) {
        let icons = if self.unicode {
            Icons::unicode()
        } else {
            Icons::ascii()
        };

        if result.is_success() && !result.has_changes() {
            println!("{} Already Up-to-date", icons.check);
            println!();
            println!(
                "  {} assets → {} targets",
                result.asset_count, result.output_count
            );
            println!("  {} files already up-to-date", result.skipped.len());
            return;
        }

        if result.is_success() {
            println!("{} Deploy Complete", icons.check);
        } else {
            println!("{} Deploy Failed", icons.cross);
        }

        println!();
        println!("  Source: {}", source.display());
        println!(
            "  {} assets → {} outputs",
            result.asset_count, result.output_count
        );
        println!();

        if !result.written.is_empty() {
            println!("  Written ({}):", result.written.len());
            for path in &result.written {
                println!("    {} {}", icons.write, path.display());
            }
        }

        if !result.skipped.is_empty() && self.verbose > 0 {
            println!("  Skipped ({}):", result.skipped.len());
            for path in &result.skipped {
                println!("    {} {}", icons.skip, path.display());
            }
        }

        if !result.deleted.is_empty() {
            println!("  Deleted ({}):", result.deleted.len());
            for path in &result.deleted {
                println!("    {} {}", icons.trash, path.display());
            }
        }

        if !result.warnings.is_empty() {
            println!();
            println!("  Warnings ({}):", result.warnings.len());
            for warning in &result.warnings {
                println!("    [!] {}", warning);
            }
        }

        if !result.errors.is_empty() {
            println!();
            println!("  Errors ({}):", result.errors.len());
            for error in &result.errors {
                println!("    {} {}", icons.cross, error);
            }
        }
    }
}

/// JSON renderer for deploy results
pub struct JsonRenderer;

impl DeployResultRenderer for JsonRenderer {
    fn render(&self, result: &DeployResult, source: &Path) {
        let json = serde_json::json!({
            "success": result.is_success(),
            "source": source.display().to_string(),
            "asset_count": result.asset_count,
            "output_count": result.output_count,
            "written": result.written.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "skipped": result.skipped.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "deleted": result.deleted.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
            "warnings": result.warnings,
            "errors": result.errors,
        });

        println!(
            "{}",
            serde_json::to_string_pretty(&json).unwrap_or_default()
        );
    }
}

/// Create a renderer based on format
pub fn create_renderer(
    format: OutputFormat,
    color: bool,
    unicode: bool,
    verbose: u8,
) -> Box<dyn DeployResultRenderer> {
    match format {
        OutputFormat::Text | OutputFormat::Minimal => Box::new(TextRenderer {
            color,
            unicode,
            verbose,
        }),
        OutputFormat::Json => Box::new(JsonRenderer),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_default_is_text() {
        assert_eq!(OutputFormat::default(), OutputFormat::Text);
    }

    #[test]
    fn text_renderer_default_has_color_and_unicode() {
        let renderer = TextRenderer::default();
        assert!(renderer.color);
        assert!(renderer.unicode);
    }

    #[test]
    fn create_renderer_returns_text_for_text_format() {
        let _renderer = create_renderer(OutputFormat::Text, true, true, 0);
        // Type check passes
    }

    #[test]
    fn create_renderer_returns_json_for_json_format() {
        let _renderer = create_renderer(OutputFormat::Json, true, true, 0);
        // Type check passes
    }

    #[test]
    fn icons_unicode() {
        let icons = Icons::unicode();
        assert_eq!(icons.check, "✓");
    }

    #[test]
    fn icons_ascii() {
        let icons = Icons::ascii();
        assert_eq!(icons.check, "[OK]");
    }
}
