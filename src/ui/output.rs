use std::path::Path;

use crate::ui::blocks::warning::WarningBlock;
use crate::ui::context::UiContext;
use crate::ui::primitives::icon::Icon;

/// Warn if security.allow_naked is set to true
/// Currently not wired up - reserved for future use
#[allow(dead_code)]
pub fn maybe_warn_allow_naked(config: &calvin::config::Config, ui: &UiContext) {
    if !config.security.allow_naked {
        return;
    }

    if ui.json {
        let _ = crate::ui::json::emit(serde_json::json!({
            "event": "warning",
            "kind": "allow_naked",
            "message": "Security protections disabled!",
            "config_key": "security.allow_naked",
            "value": true
        }));
        return;
    }

    if ui.caps.is_ci && std::env::var("GITHUB_ACTIONS").is_ok() {
        println!(
            "{}",
            crate::ui::ci::github_actions_annotation(
                crate::ui::ci::AnnotationLevel::Warning,
                "security.allow_naked = true (security protections disabled)",
                None,
                None,
                Some("Calvin"),
            )
        );
    }

    let rendered = format_allow_naked_warning(ui.color, ui.unicode);
    eprint!("{rendered}");
}

#[allow(dead_code)]
fn format_allow_naked_warning(supports_color: bool, supports_unicode: bool) -> String {
    let mut block = WarningBlock::new("Security protections disabled!");
    block.add_line("You have set security.allow_naked = true.");
    block.add_line(".env, private keys, and .git may be visible to AI assistants.");
    block.add_line("This is your responsibility.");
    block.render(supports_color, supports_unicode)
}

pub fn print_config_warnings(path: &Path, warnings: &[calvin::config::ConfigWarning], ui: &UiContext) {
    if warnings.is_empty() {
        return;
    }

    if ui.json {
        let mut out = std::io::stdout().lock();
        for warning in warnings {
            let _ = crate::ui::json::write_event(
                &mut out,
                &serde_json::json!({
                    "event": "warning",
                    "kind": "unknown_config_key",
                    "key": warning.key,
                    "file": warning.file.display().to_string(),
                    "line": warning.line,
                    "suggestion": warning.suggestion
                }),
            );
        }
        return;
    }

    if ui.caps.is_ci && std::env::var("GITHUB_ACTIONS").is_ok() {
        for warning in warnings {
            let mut msg = format!("Unknown config key '{}'", warning.key);
            if let Some(suggestion) = &warning.suggestion {
                msg.push_str(&format!(" (did you mean '{}'?)", suggestion));
            }
            println!(
                "{}",
                crate::ui::ci::github_actions_annotation(
                    crate::ui::ci::AnnotationLevel::Warning,
                    &msg,
                    Some(&warning.file.display().to_string()),
                    warning.line,
                    Some("Calvin config"),
                )
            );
        }
    }

    let rendered = format_config_warnings(path, warnings, ui.color, ui.unicode);
    if rendered.is_empty() {
        return;
    }
    eprint!("{rendered}");
}

pub fn format_config_warnings(
    path: &Path,
    warnings: &[calvin::config::ConfigWarning],
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let mut out = String::new();
    for warning in warnings {
        out.push_str(&format_config_warning(
            path,
            warning,
            supports_color,
            supports_unicode,
        ));
    }
    out
}

fn format_config_warning(
    path: &Path,
    warning: &calvin::config::ConfigWarning,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let icon = Icon::Warning.colored(supports_color, supports_unicode);

    let mut out = String::new();
    out.push_str(&icon);
    out.push_str(&format!(" Unknown config key '{}' in {}", warning.key, path.display()));
    if let Some(line) = warning.line {
        out.push_str(&format!(":{}", line));
    }
    out.push('\n');

    if let Some(suggestion) = &warning.suggestion {
        let arrow = Icon::Arrow.colored(supports_color, supports_unicode);
        out.push_str(&format!("  {} Did you mean '{}'?\n\n", arrow, suggestion));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn allow_naked_warning_has_actionable_key() {
        let rendered = format_allow_naked_warning(false, false);
        assert!(rendered.contains("security.allow_naked = true"));
    }

    #[test]
    fn config_warning_includes_suggestion_when_available() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let warning = calvin::config::ConfigWarning {
            key: "targtes".to_string(),
            file: path.clone(),
            line: Some(12),
            suggestion: Some("targets".to_string()),
        };

        let rendered = format_config_warnings(&path, &[warning], false, false);
        assert!(rendered.contains("[>] Did you mean 'targets'?"));
    }
}
