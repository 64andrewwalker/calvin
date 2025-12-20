//! Security Mode Value Object
//!
//! Defines the security enforcement level for Calvin operations.
//! This is a core domain concept used by SecurityPolicy.

use serde::{Deserialize, Serialize};

/// Security mode for Calvin operations
///
/// Determines how strictly security rules are enforced:
/// - `Yolo`: No enforcement, INFO logs only
/// - `Balanced`: Generate protections, WARN on issues (default)
/// - `Strict`: Block on security violations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SecurityMode {
    /// No enforcement, INFO logs only
    Yolo,
    /// Generate protections, WARN on issues (default)
    #[default]
    Balanced,
    /// Block on security violations
    Strict,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_balanced() {
        assert_eq!(SecurityMode::default(), SecurityMode::Balanced);
    }

    #[test]
    fn serde_roundtrip() {
        let modes = [
            SecurityMode::Yolo,
            SecurityMode::Balanced,
            SecurityMode::Strict,
        ];

        for mode in modes {
            let json = serde_json::to_string(&mode).unwrap();
            let parsed: SecurityMode = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, mode);
        }
    }

    #[test]
    fn serde_lowercase() {
        let json = r#""strict""#;
        let mode: SecurityMode = serde_json::from_str(json).unwrap();
        assert_eq!(mode, SecurityMode::Strict);
    }
}
