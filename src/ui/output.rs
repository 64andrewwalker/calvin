use std::path::Path;

use crate::ui::context::UiContext;
use crate::ui::primitives::icon::Icon;

pub fn print_config_warnings(
    path: &Path,
    warnings: &[calvin::domain::value_objects::ConfigWarning],
    ui: &UiContext,
) {
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
    warnings: &[calvin::domain::value_objects::ConfigWarning],
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
    warning: &calvin::domain::value_objects::ConfigWarning,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    let icon = Icon::Warning.colored(supports_color, supports_unicode);

    let mut out = String::new();
    out.push_str(&icon);
    out.push_str(&format!(
        " Unknown config key '{}' in {}",
        warning.key,
        path.display()
    ));
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
    fn config_warning_includes_suggestion_when_available() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let warning = calvin::domain::value_objects::ConfigWarning {
            key: "targtes".to_string(),
            file: path.clone(),
            line: Some(12),
            suggestion: Some("targets".to_string()),
        };

        let rendered = format_config_warnings(&path, &[warning], false, false);
        assert!(rendered.contains("[>] Did you mean 'targets'?"));
    }
}
