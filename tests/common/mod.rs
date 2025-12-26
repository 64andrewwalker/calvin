//! Common test utilities for Calvin contract and scenario tests.
//!
//! This module provides:
//! - `TestEnv`: Isolated test environment with temp directories
//! - `WindowsCompatExt`: Extension trait for Windows-compatible Command setup
//! - Assertion macros: `assert_deployed!`, `assert_output_contains!`, etc.
//! - Fixtures: Reusable test content constants

pub mod assertions;
pub mod env;
pub mod fixtures;
pub mod windows;

pub use assertions::*;
pub use env::*;
pub use fixtures::*;
pub use windows::*;
