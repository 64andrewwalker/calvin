use crossterm::style::Color;
use dialoguer::theme::Theme;
use std::fmt;

/// Design tokens for Calvin CLI UI.
///
/// Design constraints:
/// - Only 5 semantic colors (`colors::*`)
/// - All icons and borders must be sourced from this module
pub mod colors {
    use super::Color;

    /// #22C55E
    pub const SUCCESS: Color = Color::Green;
    /// #EF4444
    pub const ERROR: Color = Color::Red;
    /// #F59E0B
    pub const WARNING: Color = Color::Yellow;
    /// #06B6D4
    pub const INFO: Color = Color::Cyan;
    /// #6B7280
    pub const DIM: Color = Color::DarkGrey;
}

#[allow(dead_code)] // Future icons for calvin clean
pub mod icons {
    pub const SUCCESS: &str = "✓";
    pub const ERROR: &str = "✗";
    pub const WARNING: &str = "⚠";
    pub const PROGRESS: &str = "●";
    pub const PENDING: &str = "○";
    pub const ARROW: &str = "↳";
    pub const POINTER: &str = "↑";

    // Selection states (for MultiSelect).
    pub const SELECTED: &str = "●";
    pub const UNSELECTED: &str = "○";
    pub const PARTIAL: &str = "◐";

    // Tree expansion.
    pub const EXPAND: &str = "▼";
    pub const COLLAPSE: &str = "▶";

    // Command identifiers (used in headers).
    pub const WATCH: &str = "⟳";
    pub const DEPLOY: &str = "📦";
    pub const CHECK: &str = "🔍";
    pub const REMOTE: &str = "📡";
    pub const DIFF: &str = "Δ";
    pub const TRASH: &str = "🗑";
    pub const CLEAN: &str = "🧹";
}

#[allow(dead_code)] // Future icons for calvin clean
pub mod icons_ascii {
    pub const SUCCESS: &str = "[OK]";
    pub const ERROR: &str = "[FAIL]";
    pub const WARNING: &str = "[WARN]";
    pub const PROGRESS: &str = "[..]";
    pub const PENDING: &str = "[ ]";
    pub const ARROW: &str = "[>]";
    pub const POINTER: &str = "^";

    // Selection states (for MultiSelect).
    pub const SELECTED: &str = "[x]";
    pub const UNSELECTED: &str = "[ ]";
    pub const PARTIAL: &str = "[-]";

    // Tree expansion.
    pub const EXPAND: &str = "[v]";
    pub const COLLAPSE: &str = "[>]";

    pub const WATCH: &str = "[~]";
    pub const DEPLOY: &str = "[DEPLOY]";
    pub const CHECK: &str = "[CHECK]";
    pub const REMOTE: &str = "[REMOTE]";
    pub const DIFF: &str = "[DIFF]";
    pub const TRASH: &str = "[DEL]";
    pub const CLEAN: &str = "[CLEAN]";
}

pub mod borders {
    pub const TOP_LEFT: &str = "╭";
    pub const TOP_RIGHT: &str = "╮";
    pub const BOTTOM_LEFT: &str = "╰";
    pub const BOTTOM_RIGHT: &str = "╯";
    pub const HORIZONTAL: &str = "─";
    pub const VERTICAL: &str = "│";
}

pub mod borders_ascii {
    pub const TOP_LEFT: &str = "+";
    pub const TOP_RIGHT: &str = "+";
    pub const BOTTOM_LEFT: &str = "+";
    pub const BOTTOM_RIGHT: &str = "+";
    pub const HORIZONTAL: &str = "-";
    pub const VERTICAL: &str = "|";
}

// ----------------------------------------------------------------------------
// CalvinTheme - Custom dialoguer theme with ●/○ icons
// ----------------------------------------------------------------------------

/// Custom theme for dialoguer prompts using Calvin design tokens.
///
/// Uses `●` for selected items and `○` for unselected items (Unicode mode),
/// or `[x]` and `[ ]` in ASCII fallback mode.
///
/// This theme wraps `ColorfulTheme` and only overrides the multi-select item
/// formatting to use our custom icons while preserving all other behaviors
/// (including cursor visibility handling).
pub struct CalvinTheme {
    unicode: bool,
    inner: dialoguer::theme::ColorfulTheme,
}

impl CalvinTheme {
    /// Create a new CalvinTheme.
    ///
    /// # Arguments
    /// * `unicode` - Whether to use Unicode symbols (●/○) or ASCII fallback ([x]/[ ])
    pub fn new(unicode: bool) -> Self {
        Self {
            unicode,
            inner: dialoguer::theme::ColorfulTheme::default(),
        }
    }

    /// Get the icon for a selected item.
    pub fn selected_icon(&self) -> &'static str {
        if self.unicode {
            icons::SELECTED
        } else {
            icons_ascii::SELECTED
        }
    }

    /// Get the icon for an unselected item.
    pub fn unselected_icon(&self) -> &'static str {
        if self.unicode {
            icons::UNSELECTED
        } else {
            icons_ascii::UNSELECTED
        }
    }
}

impl Theme for CalvinTheme {
    fn format_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.inner.format_prompt(f, prompt)
    }

    fn format_error(&self, f: &mut dyn fmt::Write, err: &str) -> fmt::Result {
        self.inner.format_error(f, err)
    }

    fn format_confirm_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<bool>,
    ) -> fmt::Result {
        self.inner.format_confirm_prompt(f, prompt, default)
    }

    fn format_confirm_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        selection: Option<bool>,
    ) -> fmt::Result {
        self.inner
            .format_confirm_prompt_selection(f, prompt, selection)
    }

    fn format_input_prompt(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        default: Option<&str>,
    ) -> fmt::Result {
        self.inner.format_input_prompt(f, prompt, default)
    }

    fn format_input_prompt_selection(
        &self,
        f: &mut dyn fmt::Write,
        prompt: &str,
        sel: &str,
    ) -> fmt::Result {
        self.inner.format_input_prompt_selection(f, prompt, sel)
    }

    fn format_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.inner.format_select_prompt(f, prompt)
    }

    fn format_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        active: bool,
    ) -> fmt::Result {
        self.inner.format_select_prompt_item(f, text, active)
    }

    fn format_multi_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.inner.format_multi_select_prompt(f, prompt)
    }

    // This is the only method we customize for ●/○ icons
    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let icon = if checked {
            self.selected_icon()
        } else {
            self.unselected_icon()
        };

        if active {
            // Show cursor indicator for active item
            write!(f, "> {} {}", icon, text)
        } else {
            write!(f, "  {} {}", icon, text)
        }
    }

    fn format_sort_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.inner.format_sort_prompt(f, prompt)
    }

    fn format_sort_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        picked: bool,
        active: bool,
    ) -> fmt::Result {
        self.inner.format_sort_prompt_item(f, text, picked, active)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calvin_theme_unicode_icons() {
        let theme = CalvinTheme::new(true);
        assert_eq!(theme.selected_icon(), "●");
        assert_eq!(theme.unselected_icon(), "○");
    }

    #[test]
    fn test_calvin_theme_ascii_icons() {
        let theme = CalvinTheme::new(false);
        assert_eq!(theme.selected_icon(), "[x]");
        assert_eq!(theme.unselected_icon(), "[ ]");
    }
}
