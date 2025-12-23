#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(key) = std::str::from_utf8(data) {
        // Fuzz lockfile key parsing - this should never panic
        let _ = calvin::parse_lockfile_key(key);
    }
});
