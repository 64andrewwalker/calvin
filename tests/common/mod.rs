//! Common test utilities for Calvin contract and scenario tests.
//!
//! This module provides:
//! - `TestEnv`: Isolated test environment with temp directories
//! - Assertion macros: `assert_deployed!`, `assert_output_contains!`, etc.
//! - Fixtures: Reusable test content constants

pub mod assertions;
pub mod env;
pub mod fixtures;

pub use assertions::*;
pub use env::*;
pub use fixtures::*;
