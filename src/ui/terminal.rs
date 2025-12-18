#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCapabilities {
    pub is_tty: bool,
    pub supports_color: bool,
    pub supports_256_color: bool,
    pub supports_true_color: bool,
    pub supports_unicode: bool,
    pub is_ci: bool,
    pub width: u16,
    pub height: u16,
}

pub fn detect_capabilities() -> TerminalCapabilities {
    detect_capabilities_impl(
        |key| std::env::var(key).ok(),
        atty::is(atty::Stream::Stdout),
        crossterm::terminal::size().ok(),
    )
}

fn detect_capabilities_impl(
    get_env: impl Fn(&str) -> Option<String>,
    is_tty: bool,
    size: Option<(u16, u16)>,
) -> TerminalCapabilities {
    let term = get_env("TERM").unwrap_or_default();
    let term_is_dumb = term.eq_ignore_ascii_case("dumb");

    let no_color = get_env("NO_COLOR").is_some();
    let is_ci = is_ci_env(&get_env);

    let supports_color = is_tty && !term_is_dumb && !no_color;
    let supports_256_color = supports_color && term.to_lowercase().contains("256color");
    let supports_true_color = supports_color && supports_true_color_env(&get_env);
    let supports_unicode = !term_is_dumb && unicode_locale(&get_env);

    let (width, height) = size.unwrap_or((80, 24));
    TerminalCapabilities {
        is_tty,
        supports_color,
        supports_256_color,
        supports_true_color,
        supports_unicode,
        is_ci,
        width,
        height,
    }
}

fn is_ci_env(get_env: &impl Fn(&str) -> Option<String>) -> bool {
    const KEYS: &[&str] = &[
        "CI",
        "GITHUB_ACTIONS",
        "JENKINS_HOME",
        "BUILDKITE",
        "CIRCLECI",
        "TRAVIS",
        "TEAMCITY_VERSION",
    ];

    KEYS.iter().any(|k| get_env(k).is_some())
}

fn supports_true_color_env(get_env: &impl Fn(&str) -> Option<String>) -> bool {
    let colorterm = get_env("COLORTERM").unwrap_or_default().to_lowercase();
    colorterm.contains("truecolor") || colorterm.contains("24bit")
}

fn unicode_locale(get_env: &impl Fn(&str) -> Option<String>) -> bool {
    const KEYS: &[&str] = &["LC_ALL", "LC_CTYPE", "LANG"];
    for k in KEYS {
        if let Some(val) = get_env(k) {
            let v = val.to_lowercase();
            if v.contains("utf-8") || v.contains("utf8") {
                return true;
            }
        }
    }

    // Default to true on modern systems unless explicitly "dumb".
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn caps(env: &[(&str, &str)], is_tty: bool, size: Option<(u16, u16)>) -> TerminalCapabilities {
        let map: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        detect_capabilities_impl(|k| map.get(k).cloned(), is_tty, size)
    }

    #[test]
    fn detect_respects_no_color() {
        let c = caps(
            &[("NO_COLOR", "1"), ("TERM", "xterm-256color")],
            true,
            Some((120, 40)),
        );
        assert!(!c.supports_color);
    }

    #[test]
    fn detect_ci_environment() {
        let c = caps(&[("CI", "true"), ("TERM", "xterm-256color")], true, None);
        assert!(c.is_ci);
    }

    #[test]
    fn detect_term_dumb_disables_enhancements() {
        let c = caps(&[("TERM", "dumb")], true, None);
        assert!(!c.supports_color);
        assert!(!c.supports_unicode);
        assert!(!c.supports_256_color);
        assert!(!c.supports_true_color);
    }

    #[test]
    fn detect_256_color_from_term_suffix() {
        let c = caps(&[("TERM", "xterm-256color")], true, None);
        assert!(c.supports_256_color);
    }
}
