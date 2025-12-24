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

impl SecurityMode {
    /// All valid string representations for this enum
    pub const VALID_VALUES: &'static [&'static str] = &["yolo", "balanced", "strict"];

    /// Parse a string into SecurityMode (case-insensitive)
    /// Returns None for invalid values
    pub fn parse_str(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "yolo" => Some(Self::Yolo),
            "balanced" => Some(Self::Balanced),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }
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

    #[test]
    fn test_valid_values_contains_all_variants() {
        // VALID_VALUES should contain all three security modes
        assert!(SecurityMode::VALID_VALUES.contains(&"yolo"));
        assert!(SecurityMode::VALID_VALUES.contains(&"balanced"));
        assert!(SecurityMode::VALID_VALUES.contains(&"strict"));
        assert_eq!(SecurityMode::VALID_VALUES.len(), 3);
    }

    #[test]
    fn test_from_str_valid_values() {
        assert_eq!(SecurityMode::parse_str("yolo"), Some(SecurityMode::Yolo));
        assert_eq!(
            SecurityMode::parse_str("balanced"),
            Some(SecurityMode::Balanced)
        );
        assert_eq!(
            SecurityMode::parse_str("strict"),
            Some(SecurityMode::Strict)
        );
        // Case insensitive
        assert_eq!(SecurityMode::parse_str("YOLO"), Some(SecurityMode::Yolo));
        assert_eq!(
            SecurityMode::parse_str("Strict"),
            Some(SecurityMode::Strict)
        );
    }

    #[test]
    fn test_from_str_invalid_values() {
        assert_eq!(SecurityMode::parse_str("invalid"), None);
        assert_eq!(SecurityMode::parse_str(""), None);
        assert_eq!(SecurityMode::parse_str("strct"), None); // typo
    }
}
