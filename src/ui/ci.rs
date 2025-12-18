#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationLevel {
    Warning,
    Error,
}

pub fn github_actions_annotation(
    level: AnnotationLevel,
    message: &str,
    file: Option<&str>,
    line: Option<usize>,
    title: Option<&str>,
) -> String {
    let level_str = match level {
        AnnotationLevel::Warning => "warning",
        AnnotationLevel::Error => "error",
    };

    let mut props = Vec::new();
    if let Some(file) = file {
        props.push(format!("file={}", escape_workflow_command_value(file)));
    }
    if let Some(line) = line {
        props.push(format!("line={}", line));
    }
    if let Some(title) = title {
        props.push(format!("title={}", escape_workflow_command_value(title)));
    }

    let prop_str = if props.is_empty() {
        String::new()
    } else {
        format!(" {}", props.join(","))
    };

    format!(
        "::{}{}::{}",
        level_str,
        prop_str,
        escape_workflow_command_message(message)
    )
}

fn escape_workflow_command_value(s: &str) -> String {
    s.replace('%', "%25").replace('\r', "%0D").replace('\n', "%0A")
}

fn escape_workflow_command_message(s: &str) -> String {
    escape_workflow_command_value(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_actions_annotation_escapes_newlines() {
        let rendered = github_actions_annotation(
            AnnotationLevel::Error,
            "Line1\nLine2",
            Some("a/b.txt"),
            Some(3),
            Some("Title"),
        );
        assert!(rendered.contains("%0A"));
        assert!(rendered.starts_with("::error "));
    }
}

