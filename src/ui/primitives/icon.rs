use crossterm::style::Stylize;

use crate::ui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Icon {
    Success,
    Error,
    Warning,
    Progress,
    Pending,
    Arrow,
    Pointer,
    Watch,
    Deploy,
    Check,
    Remote,
    Diff,
    Trash,
}

impl Icon {
    pub fn render(&self, supports_unicode: bool) -> &'static str {
        match (supports_unicode, self) {
            (true, Icon::Success) => theme::icons::SUCCESS,
            (true, Icon::Error) => theme::icons::ERROR,
            (true, Icon::Warning) => theme::icons::WARNING,
            (true, Icon::Progress) => theme::icons::PROGRESS,
            (true, Icon::Pending) => theme::icons::PENDING,
            (true, Icon::Arrow) => theme::icons::ARROW,
            (true, Icon::Pointer) => theme::icons::POINTER,
            (true, Icon::Watch) => theme::icons::WATCH,
            (true, Icon::Deploy) => theme::icons::DEPLOY,
            (true, Icon::Check) => theme::icons::CHECK,
            (true, Icon::Remote) => theme::icons::REMOTE,
            (true, Icon::Diff) => theme::icons::DIFF,
            (true, Icon::Trash) => theme::icons::TRASH,
            (false, Icon::Success) => theme::icons_ascii::SUCCESS,
            (false, Icon::Error) => theme::icons_ascii::ERROR,
            (false, Icon::Warning) => theme::icons_ascii::WARNING,
            (false, Icon::Progress) => theme::icons_ascii::PROGRESS,
            (false, Icon::Pending) => theme::icons_ascii::PENDING,
            (false, Icon::Arrow) => theme::icons_ascii::ARROW,
            (false, Icon::Pointer) => theme::icons_ascii::POINTER,
            (false, Icon::Watch) => theme::icons_ascii::WATCH,
            (false, Icon::Deploy) => theme::icons_ascii::DEPLOY,
            (false, Icon::Check) => theme::icons_ascii::CHECK,
            (false, Icon::Remote) => theme::icons_ascii::REMOTE,
            (false, Icon::Diff) => theme::icons_ascii::DIFF,
            (false, Icon::Trash) => theme::icons_ascii::TRASH,
        }
    }

    pub fn colored(&self, supports_color: bool, supports_unicode: bool) -> String {
        let s = self.render(supports_unicode);
        if !supports_color {
            return s.to_string();
        }
        let color = match self {
            Icon::Success => theme::colors::SUCCESS,
            Icon::Error => theme::colors::ERROR,
            Icon::Warning | Icon::Progress => theme::colors::WARNING,
            Icon::Pending | Icon::Arrow => theme::colors::DIM,
            Icon::Pointer => theme::colors::ERROR,
            Icon::Watch | Icon::Deploy | Icon::Check | Icon::Remote | Icon::Diff => {
                theme::colors::INFO
            }
            Icon::Trash => theme::colors::WARNING,
        };
        format!("{}", s.with(color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_renders_ascii_when_unicode_unsupported() {
        assert_eq!(Icon::Success.render(false), theme::icons_ascii::SUCCESS);
    }

    #[test]
    fn icon_renders_unicode_when_supported() {
        assert_eq!(Icon::Warning.render(true), theme::icons::WARNING);
    }
}
