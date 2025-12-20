//! Presentation Layer
//!
//! This layer handles:
//! - CLI argument parsing (via clap)
//! - Creating use cases with infrastructure dependencies
//! - Output formatting (text/JSON)
//!
//! ## Structure
//!
//! - `cli` - CLI argument parsing and command definitions
//! - `factory` - Creates use cases with proper dependencies (dependency injection)
//! - `output` - Output rendering abstractions
//!
//! ## Usage
//!
//! ```ignore
//! use calvin::presentation::{factory, cli};
//!
//! // Parse CLI arguments
//! let cli = cli::Cli::parse();
//!
//! // Create deploy use case with all dependencies wired up
//! let use_case = factory::create_deploy_use_case();
//! let result = use_case.execute(&options);
//! ```

pub mod cli;
pub mod factory;
pub mod output;

pub use cli::{Cli, ColorWhen, Commands};
pub use factory::create_deploy_use_case;
