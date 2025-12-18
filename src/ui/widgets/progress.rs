use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressStyle {
    Bar,
    Blocks,
    Compact,
}

#[derive(Debug, Clone)]
pub struct ProgressBar {
    total: u64,
    current: u64,
    width: u16,
    message: String,
    started: Instant,
    style: ProgressStyle,
    show_eta: bool,
}

impl ProgressBar {
    pub fn new(total: u64) -> Self {
        Self {
            total,
            current: 0,
            width: 20,
            message: String::new(),
            started: Instant::now(),
            style: ProgressStyle::Bar,
            show_eta: true,
        }
    }

    pub fn with_message(total: u64, message: impl Into<String>) -> Self {
        let mut bar = Self::new(total);
        bar.message = message.into();
        bar
    }

    pub fn set_width(&mut self, width: u16) {
        self.width = width.max(1);
    }

    pub fn set_style(&mut self, style: ProgressStyle) {
        self.style = style;
    }

    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    pub fn set_show_eta(&mut self, show_eta: bool) {
        self.show_eta = show_eta;
    }

    pub fn inc(&mut self, delta: u64) {
        self.current = self.current.saturating_add(delta);
    }

    pub fn set(&mut self, value: u64) {
        self.current = value;
    }

    pub fn eta(&self) -> Option<Duration> {
        if self.total == 0 || self.current == 0 {
            return None;
        }

        if self.current >= self.total {
            return Some(Duration::from_secs(0));
        }

        let elapsed = self.started.elapsed();
        let elapsed_secs = elapsed.as_secs_f64();
        if elapsed_secs <= 0.0 {
            return None;
        }

        let rate = self.current as f64 / elapsed_secs;
        if rate <= 0.0 {
            return None;
        }

        let remaining = (self.total - self.current) as f64;
        let secs = remaining / rate;
        if !secs.is_finite() || secs.is_sign_negative() {
            return None;
        }

        Some(Duration::from_secs_f64(secs))
    }

    pub fn render(&self, supports_unicode: bool) -> String {
        let (filled, empty) = self.bar_segments();
        let bar = match (self.style, supports_unicode) {
            (ProgressStyle::Compact, _) => String::new(),
            (ProgressStyle::Bar, true) => format!("{}{}", "━".repeat(filled), "─".repeat(empty)),
            (ProgressStyle::Bar, false) => format!("{}{}", "=".repeat(filled), "-".repeat(empty)),
            (ProgressStyle::Blocks, true) => format!("{}{}", "█".repeat(filled), "░".repeat(empty)),
            (ProgressStyle::Blocks, false) => {
                format!("{}{}", "#".repeat(filled), ".".repeat(empty))
            }
        };

        let pct = if self.total == 0 {
            0
        } else {
            (self.current.saturating_mul(100)) / self.total
        };

        let mut out = String::new();
        if !self.message.is_empty() {
            out.push_str(&self.message);
            out.push(' ');
        }
        if self.style != ProgressStyle::Compact {
            out.push_str(&bar);
            out.push_str("  ");
        }
        out.push_str(&format!("{}/{} ({}%)", self.current, self.total, pct));
        if self.show_eta {
            if let Some(eta) = self.eta() {
                out.push_str(&format!("  ETA: {}", format_duration_compact(eta)));
            }
        }
        out
    }

    fn bar_segments(&self) -> (usize, usize) {
        let width = self.width.max(1) as usize;
        if self.total == 0 {
            return (0, width);
        }

        let ratio = (self.current.min(self.total)) as f64 / self.total as f64;
        let filled = (ratio * width as f64).round().clamp(0.0, width as f64) as usize;
        (filled, width.saturating_sub(filled))
    }
}

fn format_duration_compact(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 60 {
        return format!("{}s", secs);
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{}m", mins);
    }
    let hours = mins / 60;
    format!("{}h", hours)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_includes_percentage_and_counts() {
        let mut bar = ProgressBar::with_message(100, "Syncing");
        bar.width = 10;
        bar.set(50);
        let rendered = bar.render(true);
        assert!(rendered.contains("50/100"));
        assert!(rendered.contains("50%"));
    }

    #[test]
    fn eta_is_computed_when_progress_made() {
        let mut bar = ProgressBar::new(100);
        bar.started = Instant::now() - Duration::from_secs(10);
        bar.set(50);
        assert!(bar.eta().is_some());
    }

    #[test]
    fn eta_is_none_when_no_progress() {
        let mut bar = ProgressBar::new(100);
        bar.started = Instant::now() - Duration::from_secs(10);
        bar.set(0);
        assert!(bar.eta().is_none());
    }

    #[test]
    fn render_uses_ascii_characters_when_unicode_unsupported() {
        let mut bar = ProgressBar::with_message(10, "Syncing");
        bar.width = 10;
        bar.set(5);
        let rendered = bar.render(false);
        assert!(!rendered.contains('━'));
        assert!(!rendered.contains('─'));
    }

    #[test]
    fn blocks_style_renders_blocks_when_unicode_supported() {
        let mut bar = ProgressBar::with_message(10, "Syncing");
        bar.style = ProgressStyle::Blocks;
        bar.width = 10;
        bar.set(5);
        let rendered = bar.render(true);
        assert!(rendered.contains('█'));
    }

    #[test]
    fn compact_style_omits_bar() {
        let mut bar = ProgressBar::with_message(10, "Syncing");
        bar.style = ProgressStyle::Compact;
        bar.width = 10;
        bar.set(5);
        let rendered = bar.render(true);
        assert!(!rendered.contains('━'));
    }

    #[test]
    fn render_can_hide_eta() {
        let mut bar = ProgressBar::new(10);
        bar.started = Instant::now() - Duration::from_secs(10);
        bar.set(5);
        bar.set_show_eta(false);
        let rendered = bar.render(true);
        assert!(!rendered.contains("ETA:"));
    }
}
