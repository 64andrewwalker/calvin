//! Deploy target value object - domain representation of deployment destination.

use serde::{Deserialize, Serialize};

/// Deploy target configuration.
///
/// Represents where assets should be deployed to:
/// - `Unset`: Not configured, user needs to choose
/// - `Project`: Deploy to project directory
/// - `Home`: Deploy to user home directory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum DeployTarget {
    /// Not configured (user needs to choose)
    #[default]
    Unset,
    /// Deploy to project directory
    Project,
    /// Deploy to user home directory
    Home,
}
