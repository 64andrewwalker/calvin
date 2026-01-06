//! Contract tests for Calvin.
//!
//! Contracts are invariants that must ALWAYS hold.
//! A failing contract test is a P0 bug.
//!
//! Run with: cargo test --test contracts
//!
//! See: docs/testing/CONTRACT_REGISTRY.md

mod common;

#[path = "contracts/paths.rs"]
mod paths;

#[path = "contracts/layers.rs"]
mod layers;

#[path = "contracts/config.rs"]
mod config;

#[path = "contracts/calvinignore.rs"]
mod calvinignore;

#[path = "contracts/skills.rs"]
mod skills;

#[path = "contracts/agents.rs"]
mod agents;
