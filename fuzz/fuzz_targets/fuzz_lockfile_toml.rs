#![no_main]

use libfuzzer_sys::fuzz_target;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Mirror of TomlLockfile for fuzzing (private in main crate)
#[derive(Deserialize)]
struct TomlLockfile {
    #[allow(dead_code)]
    version: u32,
    #[serde(default)]
    #[allow(dead_code)]
    files: BTreeMap<String, TomlFileEntry>,
}

#[derive(Deserialize)]
struct TomlFileEntry {
    #[allow(dead_code)]
    hash: String,
}

fuzz_target!(|data: &[u8]| {
    if let Ok(content) = std::str::from_utf8(data) {
        // Fuzz lockfile TOML parsing - this should never panic
        let _ = toml::from_str::<TomlLockfile>(content);
    }
});
