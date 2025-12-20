//! Check command module
//!
//! Provides security validation for AI coding assistant configurations.

mod audit;
mod doctor;
mod engine;

pub use audit::cmd_audit;
pub use doctor::cmd_doctor;
pub use engine::cmd_check;

/// Parse security mode from string
fn parse_security_mode(
    mode: &str,
    default: calvin::config::SecurityMode,
) -> calvin::config::SecurityMode {
    use calvin::config::SecurityMode;

    match mode {
        "yolo" => SecurityMode::Yolo,
        "balanced" => SecurityMode::Balanced,
        "strict" => SecurityMode::Strict,
        _ => default,
    }
}
