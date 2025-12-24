use chrono::Utc;

use calvin::domain::entities::ProjectEntry;

use crate::ui::blocks::header::CommandHeader;
use crate::ui::primitives::icon::Icon;
use crate::ui::primitives::text::ColoredText;
use crate::ui::widgets::r#box::{Box, BoxStyle};

pub struct ProjectsView<'a> {
    projects: &'a [ProjectEntry],
    pruned: Option<&'a [std::path::PathBuf]>,
    registry_path: &'a std::path::Path,
}

impl<'a> ProjectsView<'a> {
    pub fn new(
        projects: &'a [ProjectEntry],
        pruned: Option<&'a [std::path::PathBuf]>,
        registry_path: &'a std::path::Path,
    ) -> Self {
        Self {
            projects,
            pruned,
            registry_path,
        }
    }

    pub fn render(&self, supports_color: bool, supports_unicode: bool) -> String {
        let mut out = String::new();

        let mut header = CommandHeader::new(Icon::Deploy, "Calvin Projects");
        header.add("Registry", self.registry_path.display().to_string());
        header.add("Projects", self.projects.len().to_string());
        out.push_str(&header.render(supports_color, supports_unicode));

        if let Some(pruned) = self.pruned {
            if !pruned.is_empty() {
                out.push('\n');
                for path in pruned {
                    out.push_str(&format!(
                        "{} {} {}\n",
                        Icon::Warning.colored(supports_color, supports_unicode),
                        ColoredText::warning("Removed:").render(supports_color),
                        ColoredText::dim(path.display().to_string().as_str())
                            .render(supports_color)
                    ));
                }
            }
        }

        out.push('\n');

        if self.projects.is_empty() {
            out.push_str(&format!(
                "{} {}\n\n{}\n",
                Icon::Pending.colored(supports_color, supports_unicode),
                ColoredText::dim("No projects found.").render(supports_color),
                ColoredText::dim("Run `calvin deploy` in a project to register it.")
                    .render(supports_color)
            ));
            return out;
        }

        let mut b = Box::with_title("Managed Projects").style(BoxStyle::Info);

        b.add_line(
            ColoredText::dim(format!(
                "{:<44} {:>7} {:>14}",
                "Project", "Assets", "Last Deployed"
            ))
            .render(supports_color),
        );
        b.add_line(ColoredText::dim("─".repeat(70)).render(supports_color));

        for project in self.projects {
            let status = if project.lockfile.exists() {
                Icon::Success
            } else {
                Icon::Warning
            }
            .colored(supports_color, supports_unicode);

            let project_path = truncate_middle(&project.path.display().to_string(), 42);
            let ago = humanize_ago(project.last_deployed);

            b.add_line(format!(
                "{} {:<44} {:>7} {:>14}",
                status, project_path, project.asset_count, ago
            ));
        }

        out.push_str(&b.render(supports_color, supports_unicode));

        out.push('\n');
        let mut summary = crate::ui::blocks::summary::ResultSummary::success("Projects Loaded");
        summary.add_stat("projects", self.projects.len());
        out.push_str(&summary.render(supports_color, supports_unicode));

        out
    }
}

fn humanize_ago(at: chrono::DateTime<chrono::Utc>) -> String {
    let now = Utc::now();
    let delta = now.signed_duration_since(at);
    let secs = delta.num_seconds().max(0);

    if secs < 60 {
        return "just now".to_string();
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{}m ago", mins);
    }
    let hours = mins / 60;
    if hours < 24 {
        return format!("{}h ago", hours);
    }
    let days = hours / 24;
    format!("{}d ago", days)
}

fn truncate_middle(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        return s.to_string();
    }

    if max_len <= 3 {
        return "…".repeat(max_len);
    }

    let keep = max_len.saturating_sub(1);
    let left = keep / 2;
    let right = keep.saturating_sub(left);

    let left_part: String = s.chars().take(left).collect();
    let right_part: String = s
        .chars()
        .rev()
        .take(right)
        .collect::<String>()
        .chars()
        .rev()
        .collect();

    format!("{}…{}", left_part, right_part)
}
