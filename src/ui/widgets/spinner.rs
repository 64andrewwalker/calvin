use std::time::Instant;

use crate::ui::primitives::icon::Icon;

const SPINNER_FRAMES_BRAILLE: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const SPINNER_FRAMES_DOTS: &[char] = &['⣾', '⣽', '⣻', '⢿', '⡿', '⣟', '⣯', '⣷'];
const SPINNER_FRAMES_ARROW: &[char] = &['←', '↖', '↑', '↗', '→', '↘', '↓', '↙'];
const SPINNER_FRAMES_LINE: &[char] = &['─', '╲', '│', '╱'];
const SPINNER_FRAMES_ASCII: &[char] = &['-', '\\', '|', '/'];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerStyle {
    Braille,
    Dots,
    Line,
    Arrow,
}

#[derive(Debug, Clone)]
pub struct Spinner {
    style: SpinnerStyle,
    current: usize,
    message: String,
    #[allow(dead_code)]
    started: Instant,
}

impl Spinner {
    #[allow(dead_code)]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            style: SpinnerStyle::Braille,
            current: 0,
            message: message.into(),
            started: Instant::now(),
        }
    }

    pub fn with_style(style: SpinnerStyle, message: impl Into<String>) -> Self {
        Self {
            style,
            current: 0,
            message: message.into(),
            started: Instant::now(),
        }
    }

    pub fn tick(&mut self) {
        self.current = self.current.wrapping_add(1);
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    pub fn render(&self, supports_unicode: bool) -> String {
        let frames = self.frames(supports_unicode);
        let frame = frames[self.current % frames.len()];
        format!("{} {}", frame, self.message)
    }

    pub fn succeed(self, message: &str, supports_color: bool, supports_unicode: bool) -> String {
        format!(
            "{} {}",
            Icon::Success.colored(supports_color, supports_unicode),
            message
        )
    }

    pub fn fail(self, message: &str, supports_color: bool, supports_unicode: bool) -> String {
        format!(
            "{} {}",
            Icon::Error.colored(supports_color, supports_unicode),
            message
        )
    }

    #[allow(dead_code)]
    pub fn stop(self) -> String {
        let _ = self.started;
        self.message
    }

    fn frames(&self, supports_unicode: bool) -> &'static [char] {
        if !supports_unicode {
            return SPINNER_FRAMES_ASCII;
        }

        match self.style {
            SpinnerStyle::Braille => SPINNER_FRAMES_BRAILLE,
            SpinnerStyle::Dots => SPINNER_FRAMES_DOTS,
            SpinnerStyle::Line => SPINNER_FRAMES_LINE,
            SpinnerStyle::Arrow => SPINNER_FRAMES_ARROW,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_uses_braille_frames_when_unicode_supported() {
        let s = Spinner::new("Loading");
        assert!(s.render(true).starts_with('⠋'));
    }

    #[test]
    fn render_uses_ascii_frames_when_unicode_unsupported() {
        let s = Spinner::new("Loading");
        assert!(s.render(false).starts_with('-'));
    }

    #[test]
    fn tick_advances_frame() {
        let mut s = Spinner::new("Loading");
        let first = s.render(true);
        s.tick();
        let second = s.render(true);
        assert_ne!(first, second);
    }
}
