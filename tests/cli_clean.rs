//! Integration tests for `calvin clean`.
//!
//! This test crate is intentionally split into smaller modules to keep files
//! readable during Phase 5 migration.

mod common;

#[path = "cli_clean/helpers.rs"]
mod helpers;

#[path = "cli_clean/basic.rs"]
mod basic;

#[path = "cli_clean/variants.rs"]
mod variants;
