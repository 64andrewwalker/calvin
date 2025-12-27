//! Regression tests for Calvin.
//!
//! Regression tests capture specific bugs that were found and fixed.
//! Every bug fix MUST have a regression test.
//!
//! Run with: cargo test --test regression
//!
//! See: docs/testing/TESTING_PHILOSOPHY.md

mod common;

#[path = "regression/calvinignore_deploy.rs"]
mod calvinignore_deploy;
