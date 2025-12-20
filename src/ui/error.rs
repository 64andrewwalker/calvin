use is_terminal::IsTerminal;
use std::path::Path;

pub fn format_calvin_error(err: &calvin::CalvinError) -> String {
    let caps = crate::ui::terminal::detect_capabilities();
    format_calvin_error_with(err, caps.supports_color, caps.supports_unicode)
}

fn format_calvin_error_with(
    err: &calvin::CalvinError,
    supports_color: bool,
    supports_unicode: bool,
) -> String {
    use crate::ui::blocks::error::ErrorBlock;
    use calvin::CalvinError;

    match err {
        CalvinError::MissingField { field, file, line } => {
            ErrorBlock::new(file, format!("missing required field '{}'", field))
                .with_line(*line)
                .with_file_context(2, 2)
                .with_fix(format!(
                    "Add '{}' to the frontmatter:\n  ---\n  {}: \"Your description here\"\n  ---",
                    field, field
                ))
                .render(supports_color, supports_unicode)
        }
        CalvinError::NoFrontmatter { file } => ErrorBlock::new(
            file,
            "No YAML frontmatter found. Files must start with '---'.",
        )
        .with_file_context(0, 3)
        .with_fix(
            "Add frontmatter at the top:\n  ---\n  description: \"Your description here\"\n  ---",
        )
        .render(supports_color, supports_unicode),
        CalvinError::UnclosedFrontmatter { file } => {
            ErrorBlock::new(file, "Frontmatter is not properly closed.")
                .with_fix("Add a closing '---' line after the YAML frontmatter.")
                .render(supports_color, supports_unicode)
        }
        CalvinError::InvalidFrontmatter { file, message } => ErrorBlock::new(file, message)
            .with_fix("Fix the YAML frontmatter and try again.")
            .render(supports_color, supports_unicode),
        other => {
            let file = error_file(other)
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("Calvin error"));
            ErrorBlock::new(file, other.to_string()).render(supports_color, supports_unicode)
        }
    }
}

pub fn format_error(err: &anyhow::Error) -> String {
    if let Some(calvin) = err.downcast_ref::<calvin::CalvinError>() {
        return format_calvin_error(calvin);
    }

    format!("[ERROR] {}\n", err)
}

pub fn print_error(err: &anyhow::Error, json: bool) {
    if json {
        let output = serde_json::json!({
            "event": "error",
            "message": err.to_string(),
        });
        let _ = crate::ui::json::emit(output);
        return;
    }

    let caps = crate::ui::terminal::detect_capabilities();
    if caps.is_ci && std::env::var("GITHUB_ACTIONS").is_ok() {
        let (file, line) = match err.downcast_ref::<calvin::CalvinError>() {
            Some(calvin) => match calvin {
                calvin::CalvinError::MissingField { file, line, .. } => {
                    (Some(file.as_path()), Some(*line))
                }
                calvin::CalvinError::InvalidFrontmatter { file, .. } => {
                    (Some(file.as_path()), None)
                }
                calvin::CalvinError::NoFrontmatter { file } => (Some(file.as_path()), None),
                calvin::CalvinError::UnclosedFrontmatter { file } => (Some(file.as_path()), None),
                _ => (error_file(calvin), None),
            },
            None => (None, None),
        };

        let file_str = file.map(|p| p.to_string_lossy().to_string());
        println!(
            "{}",
            crate::ui::ci::github_actions_annotation(
                crate::ui::ci::AnnotationLevel::Error,
                &err.to_string(),
                file_str.as_deref(),
                line,
                Some("Calvin"),
            )
        );
    }

    eprint!("{}", format_error(err));
}

pub fn offer_open_in_editor(err: &anyhow::Error, json: bool) {
    if json || !std::io::stdin().is_terminal() {
        return;
    }

    let (file, line) = match err.downcast_ref::<calvin::CalvinError>() {
        Some(calvin) => match calvin {
            calvin::CalvinError::MissingField { file, line, .. } => {
                (Some(file.as_path()), Some(*line))
            }
            calvin::CalvinError::InvalidFrontmatter { file, .. } => (Some(file.as_path()), None),
            calvin::CalvinError::NoFrontmatter { file } => (Some(file.as_path()), None),
            calvin::CalvinError::UnclosedFrontmatter { file } => (Some(file.as_path()), None),
            _ => (error_file(calvin), None),
        },
        None => (None, None),
    };

    let Some(file) = file else { return };

    let editor = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .ok();
    let Some(editor) = editor else { return };

    eprintln!("\n? Press Enter to open this file in your editor, or Ctrl+C to exit");
    let mut input = String::new();
    let _ = std::io::stdin().read_line(&mut input);
    if !input.trim().is_empty() {
        return;
    }

    if let Err(e) = open_in_editor(&editor, file, line) {
        eprintln!("[WARN] Failed to open editor: {}", e);
    }
}

fn open_in_editor(editor: &str, file: &Path, line: Option<usize>) -> std::io::Result<()> {
    use std::process::Command;

    let mut parts = editor.split_whitespace();
    let program = match parts.next() {
        Some(program) => program,
        None => return Ok(()),
    };
    let mut args: Vec<String> = parts.map(|s| s.to_string()).collect();

    let program_name = std::path::Path::new(program)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(program)
        .to_lowercase();

    if let Some(line) = line {
        if program_name == "code" || program_name == "code-insiders" || program_name == "cursor" {
            args.push("-g".to_string());
            args.push(format!("{}:{}", file.display(), line));
        } else if program_name == "vim" || program_name == "nvim" || program_name == "vi" {
            args.push(format!("+{}", line));
            args.push(file.display().to_string());
        } else {
            args.push(file.display().to_string());
        }
    } else {
        args.push(file.display().to_string());
    }

    Command::new(program).args(args).status()?;
    Ok(())
}

fn error_file(err: &calvin::CalvinError) -> Option<&Path> {
    use calvin::CalvinError;

    match err {
        CalvinError::MissingField { file, .. } => Some(file.as_path()),
        CalvinError::InvalidFrontmatter { file, .. } => Some(file.as_path()),
        CalvinError::NoFrontmatter { file } => Some(file.as_path()),
        CalvinError::UnclosedFrontmatter { file } => Some(file.as_path()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::primitives::icon::Icon;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_format_missing_field_includes_error_header_and_fix() {
        let err = calvin::CalvinError::MissingField {
            field: "description".to_string(),
            file: PathBuf::from(".promptpack/actions/test.md"),
            line: 2,
        };

        let rendered = format_calvin_error_with(&err, false, true);
        assert!(rendered.contains("ERROR"));
        assert!(rendered.contains("FIX:"));
    }

    #[test]
    fn test_format_no_frontmatter_includes_fix() {
        let err = calvin::CalvinError::NoFrontmatter {
            file: PathBuf::from(".promptpack/actions/test.md"),
        };

        let rendered = format_calvin_error_with(&err, false, true);
        assert!(rendered.contains("No YAML frontmatter"));
        assert!(rendered.contains("FIX:"));
    }

    #[test]
    fn test_format_missing_field_includes_code_context_when_file_exists() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("action.md");
        std::fs::write(&file, "first\nsecond\nthird\n").unwrap();

        let err = calvin::CalvinError::MissingField {
            field: "description".to_string(),
            file: file.clone(),
            line: 2,
        };

        let rendered = format_calvin_error_with(&err, false, true);
        assert!(rendered.contains(&format!("{}    2 | second", Icon::Pointer.render(true))));
    }

    #[test]
    fn test_format_unclosed_frontmatter() {
        let err = calvin::CalvinError::UnclosedFrontmatter {
            file: PathBuf::from("test.md"),
        };

        let rendered = format_calvin_error_with(&err, false, true);
        assert!(rendered.contains("not properly closed"));
        assert!(rendered.contains("FIX:"));
    }

    #[test]
    fn test_format_invalid_frontmatter() {
        let err = calvin::CalvinError::InvalidFrontmatter {
            file: PathBuf::from("test.md"),
            message: "Invalid YAML syntax".to_string(),
        };

        let rendered = format_calvin_error_with(&err, false, true);
        assert!(rendered.contains("Invalid YAML syntax"));
    }

    #[test]
    fn test_format_io_error() {
        let err = calvin::CalvinError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "File not found",
        ));

        let rendered = format_calvin_error_with(&err, false, true);
        assert!(rendered.contains("File not found"));
    }

    #[test]
    fn test_format_error_with_anyhow() {
        let err = anyhow::anyhow!("Generic error message");
        let rendered = format_error(&err);
        assert!(rendered.contains("Generic error message"));
        assert!(rendered.contains("[ERROR]"));
    }

    #[test]
    fn test_format_error_with_calvin_error() {
        let calvin_err = calvin::CalvinError::NoFrontmatter {
            file: PathBuf::from("test.md"),
        };
        let err = anyhow::Error::from(calvin_err);
        let rendered = format_error(&err);
        assert!(rendered.contains("No YAML frontmatter"));
    }

    #[test]
    fn test_error_file_extraction() {
        let missing = calvin::CalvinError::MissingField {
            field: "test".to_string(),
            file: PathBuf::from("a.md"),
            line: 1,
        };
        assert_eq!(error_file(&missing), Some(Path::new("a.md")));

        let no_fm = calvin::CalvinError::NoFrontmatter {
            file: PathBuf::from("b.md"),
        };
        assert_eq!(error_file(&no_fm), Some(Path::new("b.md")));

        let unclosed = calvin::CalvinError::UnclosedFrontmatter {
            file: PathBuf::from("c.md"),
        };
        assert_eq!(error_file(&unclosed), Some(Path::new("c.md")));

        let invalid = calvin::CalvinError::InvalidFrontmatter {
            file: PathBuf::from("d.md"),
            message: "".to_string(),
        };
        assert_eq!(error_file(&invalid), Some(Path::new("d.md")));

        let io = calvin::CalvinError::Io(std::io::Error::new(std::io::ErrorKind::Other, "error"));
        assert_eq!(error_file(&io), None);
    }

    #[test]
    fn test_format_with_color_support() {
        let err = calvin::CalvinError::NoFrontmatter {
            file: PathBuf::from("test.md"),
        };

        // With color support
        let with_color = format_calvin_error_with(&err, true, true);
        // Without color support
        let without_color = format_calvin_error_with(&err, false, true);

        // Both should contain the error message
        assert!(with_color.contains("No YAML frontmatter"));
        assert!(without_color.contains("No YAML frontmatter"));
    }

    #[test]
    fn test_format_with_unicode_support() {
        let err = calvin::CalvinError::NoFrontmatter {
            file: PathBuf::from("test.md"),
        };

        // With unicode
        let with_unicode = format_calvin_error_with(&err, false, true);
        // Without unicode
        let without_unicode = format_calvin_error_with(&err, false, false);

        // Both should contain the error message
        assert!(with_unicode.contains("No YAML frontmatter"));
        assert!(without_unicode.contains("No YAML frontmatter"));
    }
}
