//! Property tests for Calvin.
//!
//! Properties use randomized input generation to explore edge cases and
//! protect invariants like "never panics" and "round-trips".
//!
//! Run with: `cargo test --test properties`
//!
//! See: docs/testing/TESTING_PHILOSOPHY.md

#[path = "properties/frontmatter.rs"]
mod frontmatter;

#[path = "properties/lockfile_keys.rs"]
mod lockfile_keys;

#[path = "properties/layer_resolver.rs"]
mod layer_resolver;

#[path = "properties/path_handling.rs"]
mod path_handling;
