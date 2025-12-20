use crate::ui::terminal::{detect_capabilities, TerminalCapabilities};
use calvin::presentation::ColorWhen;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiContext {
    pub json: bool,
    pub verbose: u8,
    pub caps: TerminalCapabilities,
    pub color: bool,
    pub unicode: bool,
    pub animation: bool,
}

impl UiContext {
    pub fn new(
        json: bool,
        verbose: u8,
        cli_color: Option<ColorWhen>,
        cli_no_animation: bool,
        config: &calvin::config::Config,
    ) -> Self {
        let caps = detect_capabilities();
        Self::from_caps(json, verbose, cli_color, cli_no_animation, config, caps)
    }

    pub(crate) fn from_caps(
        json: bool,
        verbose: u8,
        cli_color: Option<ColorWhen>,
        cli_no_animation: bool,
        config: &calvin::config::Config,
        caps: TerminalCapabilities,
    ) -> Self {
        let unicode = config.output.unicode && caps.supports_unicode;

        let color = match cli_color {
            Some(ColorWhen::Never) => false,
            Some(ColorWhen::Always) => true,
            Some(ColorWhen::Auto) | None => match config.output.color {
                calvin::config::ColorMode::Never => false,
                calvin::config::ColorMode::Always => true,
                calvin::config::ColorMode::Auto => caps.supports_color && !caps.is_ci,
            },
        };

        let animation = if json || cli_no_animation || caps.is_ci {
            false
        } else {
            match config.output.animation {
                calvin::config::AnimationMode::Never => false,
                calvin::config::AnimationMode::Always => caps.is_tty,
                calvin::config::AnimationMode::Minimal => false,
                calvin::config::AnimationMode::Auto => caps.is_tty && !caps.is_ci,
            }
        };

        Self {
            json,
            verbose,
            caps,
            color,
            unicode,
            animation,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ci_caps() -> TerminalCapabilities {
        TerminalCapabilities {
            is_tty: true,
            supports_color: true,
            supports_256_color: false,
            supports_true_color: false,
            supports_unicode: true,
            is_ci: true,
            width: 120,
            height: 40,
        }
    }

    #[test]
    fn ci_forces_animation_off_even_when_config_is_always() {
        let mut config = calvin::config::Config::default();
        config.output.animation = calvin::config::AnimationMode::Always;

        let ui = UiContext::from_caps(false, 0, None, false, &config, ci_caps());
        assert!(!ui.animation);
    }

    #[test]
    fn ci_defaults_to_no_color_when_auto() {
        let mut config = calvin::config::Config::default();
        config.output.color = calvin::config::ColorMode::Auto;

        let ui = UiContext::from_caps(false, 0, None, false, &config, ci_caps());
        assert!(!ui.color);
    }

    #[test]
    fn ci_allows_explicit_color_always_flag() {
        let config = calvin::config::Config::default();
        let ui = UiContext::from_caps(false, 0, Some(ColorWhen::Always), false, &config, ci_caps());
        assert!(ui.color);
    }
}
