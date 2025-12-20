//! Tests for the interactive module

use super::wizard::{write_config, write_promptpack, SecurityChoice, TemplateChoice};
use tempfile::tempdir;

#[test]
fn test_write_promptpack_creates_config_and_templates() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");

    write_promptpack(
        &promptpack,
        &[calvin::Target::ClaudeCode, calvin::Target::Cursor],
        vec![TemplateChoice::Review],
        SecurityChoice::Balanced,
    )
    .unwrap();

    let config = std::fs::read_to_string(promptpack.join("config.toml")).unwrap();
    assert!(config.contains("[targets]"));
    assert!(config.contains("claude-code"));
    assert!(config.contains("cursor"));
    assert!(config.contains("mode = \"balanced\""));

    let review = std::fs::read_to_string(promptpack.join("actions/review.md")).unwrap();
    assert!(review.contains("description: PR review helper"));
}

#[test]
fn test_write_promptpack_empty_templates_creates_no_action_files() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");

    write_promptpack(
        &promptpack,
        &[calvin::Target::ClaudeCode],
        vec![TemplateChoice::Empty],
        SecurityChoice::Balanced,
    )
    .unwrap();

    let entries = std::fs::read_dir(promptpack.join("actions")).unwrap();
    let count = entries.count();
    assert_eq!(count, 0);
}

#[test]
fn test_write_config_does_not_overwrite_existing_file() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    let config_path = promptpack.join("config.toml");
    std::fs::write(&config_path, "sentinel\n").unwrap();

    write_config(
        &promptpack,
        &[calvin::Target::ClaudeCode],
        SecurityChoice::Strict,
    )
    .unwrap();

    let config = std::fs::read_to_string(config_path).unwrap();
    assert_eq!(config, "sentinel\n");
}

#[test]
fn test_write_config_minimal_sets_allow_naked_true() {
    let dir = tempdir().unwrap();
    let promptpack = dir.path().join(".promptpack");
    std::fs::create_dir_all(&promptpack).unwrap();

    write_config(
        &promptpack,
        &[calvin::Target::ClaudeCode],
        SecurityChoice::Minimal,
    )
    .unwrap();

    let config = std::fs::read_to_string(promptpack.join("config.toml")).unwrap();
    assert!(config.contains("allow_naked = true"));
}
