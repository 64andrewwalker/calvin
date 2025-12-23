#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(content) = std::str::from_utf8(data) {
        // Fuzz TOML config parsing - this should never panic
        let _ = toml::from_str::<calvin::Config>(content);
    }
});
