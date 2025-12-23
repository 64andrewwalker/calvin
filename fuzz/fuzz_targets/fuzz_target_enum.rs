#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(content) = std::str::from_utf8(data) {
        // Fuzz Target enum deserialization from YAML
        let _ = calvin::serde_yaml_ng::from_str::<calvin::Target>(content);
        
        // Fuzz Target enum deserialization from JSON
        let _ = serde_json::from_str::<calvin::Target>(content);
    }
});
