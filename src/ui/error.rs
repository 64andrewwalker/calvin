use std::path::Path;

pub fn format_calvin_error(err: &calvin::CalvinError) -> String {
    use calvin::CalvinError;

    let (title, location, body, fix) = match err {
        CalvinError::MissingField { field, file, line } => (
            file.display().to_string(),
            Some(format!(
                "Line {}: missing required field '{}'",
                line,
                field
            )),
            "Every prompt file needs YAML frontmatter with a description.".to_string(),
            Some(format!(
                "Add '{}' to the frontmatter:\n  ---\n  {}: \"Your description here\"\n  ---",
                field, field
            )),
        ),
        CalvinError::NoFrontmatter { file } => (
            file.display().to_string(),
            None,
            "No YAML frontmatter found. Files must start with '---'.".to_string(),
            Some("Add frontmatter at the top:\n  ---\n  description: \"Your description here\"\n  ---".to_string()),
        ),
        CalvinError::UnclosedFrontmatter { file } => (
            file.display().to_string(),
            None,
            "Frontmatter is not properly closed.".to_string(),
            Some("Add a closing '---' line after the YAML frontmatter.".to_string()),
        ),
        CalvinError::InvalidFrontmatter { file, message } => (
            file.display().to_string(),
            None,
            message.clone(),
            Some("Fix the YAML frontmatter and try again.".to_string()),
        ),
        other => (
            "Calvin error".to_string(),
            error_file(other).map(|p| p.display().to_string()),
            other.to_string(),
            None,
        ),
    };

    let mut rendered = String::new();
    rendered.push_str(&format!("[ERROR] {}\n", title));
    if let Some(location) = location {
        rendered.push_str(&format!("        {}\n", location));
    }
    rendered.push('\n');
    rendered.push_str(&body);
    rendered.push('\n');
    if let Some(fix) = fix {
        rendered.push('\n');
        rendered.push_str("FIX: ");
        rendered.push_str(&fix);
        rendered.push('\n');
    }
    rendered
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
        println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|_| "{}".to_string()));
        return;
    }

    eprintln!("{}", format_error(err));
}

pub fn offer_open_in_editor(err: &anyhow::Error, json: bool) {
    if json || !atty::is(atty::Stream::Stdin) {
        return;
    }

    let (file, line) = match err.downcast_ref::<calvin::CalvinError>() {
        Some(calvin) => match calvin {
            calvin::CalvinError::MissingField { file, line, .. } => (Some(file.as_path()), Some(*line)),
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
    use std::path::PathBuf;

    #[test]
    fn test_format_missing_field_includes_error_header_and_fix() {
        let err = calvin::CalvinError::MissingField {
            field: "description".to_string(),
            file: PathBuf::from(".promptpack/actions/test.md"),
            line: 2,
        };

        let rendered = format_calvin_error(&err);
        assert!(rendered.contains("[ERROR]"));
        assert!(rendered.contains("FIX:"));
    }

    #[test]
    fn test_format_no_frontmatter_includes_fix() {
        let err = calvin::CalvinError::NoFrontmatter {
            file: PathBuf::from(".promptpack/actions/test.md"),
        };

        let rendered = format_calvin_error(&err);
        assert!(rendered.contains("No YAML frontmatter"));
        assert!(rendered.contains("FIX:"));
    }
}
