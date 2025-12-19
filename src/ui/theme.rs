use crossterm::style::Color;

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

pub mod icons {
    pub const SUCCESS: &str = "✓";
    pub const ERROR: &str = "✗";
    pub const WARNING: &str = "⚠";
    pub const PROGRESS: &str = "●";
    pub const PENDING: &str = "○";
    pub const ARROW: &str = "↳";
    pub const POINTER: &str = "↑";

    // Command identifiers (used in headers).
    pub const WATCH: &str = "⟳";
    pub const DEPLOY: &str = "📦";
    pub const CHECK: &str = "🔍";
    pub const REMOTE: &str = "📡";
    pub const DIFF: &str = "Δ";
    pub const TRASH: &str = "🗑";
}

pub mod icons_ascii {
    pub const SUCCESS: &str = "[OK]";
    pub const ERROR: &str = "[FAIL]";
    pub const WARNING: &str = "[WARN]";
    pub const PROGRESS: &str = "[..]";
    pub const PENDING: &str = "[ ]";
    pub const ARROW: &str = "[>]";
    pub const POINTER: &str = "^";

    pub const WATCH: &str = "[~]";
    pub const DEPLOY: &str = "[DEPLOY]";
    pub const CHECK: &str = "[CHECK]";
    pub const REMOTE: &str = "[REMOTE]";
    pub const DIFF: &str = "[DIFF]";
    pub const TRASH: &str = "[DEL]";
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
