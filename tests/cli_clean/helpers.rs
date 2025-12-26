use std::path::Path;

use crate::common::*;
use sha2::{Digest, Sha256};

/// Normalize path for use in lockfile keys (Windows uses backslashes).
pub fn normalize_path_for_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn sha256_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub fn fresh_env() -> TestEnv {
    TestEnv::builder().fresh_environment().build()
}

pub fn deployed_project_env() -> TestEnv {
    let env = TestEnv::builder()
        .with_project_asset("test.md", SIMPLE_POLICY)
        .with_project_config(CONFIG_DEPLOY_PROJECT)
        .build();

    let result = env.run(&["deploy", "--yes"]);
    assert!(
        result.success,
        "Deploy failed:\n{}",
        result.combined_output()
    );

    env
}

pub fn write_project_config(env: &TestEnv, toml: &str) {
    env.write_project_file(".promptpack/config.toml", toml);
}

pub fn write_legacy_lockfile(env: &TestEnv, content: &str) {
    env.write_project_file(".promptpack/.calvin.lock", content);
}
