//! Security check types

/// Security check result
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityCheck {
    pub name: String,
    pub platform: String,
    pub status: CheckStatus,
    pub message: String,
    pub recommendation: Option<String>,
    pub details: Vec<String>,
}

/// Status of a security check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    Pass,
    Warning,
    Error,
}

impl std::fmt::Display for CheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CheckStatus::Pass => write!(f, "✓"),
            CheckStatus::Warning => write!(f, "⚠"),
            CheckStatus::Error => write!(f, "✗"),
        }
    }
}
