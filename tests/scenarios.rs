//! Scenario tests for Calvin.
//!
//! Scenarios test complete user workflows end-to-end.
//! Each scenario represents a real user journey.
//!
//! Run with: cargo test --test scenarios
//!
//! See: docs/testing/TESTING_PHILOSOPHY.md

mod common;

#[path = "scenarios/first_time_user.rs"]
mod first_time_user;

#[path = "scenarios/team_shared.rs"]
mod team_shared;

#[path = "scenarios/clean_all.rs"]
mod clean_all;
