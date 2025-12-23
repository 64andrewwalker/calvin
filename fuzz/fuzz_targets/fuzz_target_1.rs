#![no_main]

use libfuzzer_sys::fuzz_target;
use std::path::Path;

fuzz_target!(|data: &[u8]| {
    // Try to convert bytes to a valid UTF-8 string
    if let Ok(content) = std::str::from_utf8(data) {
        let fake_path = Path::new("fuzz.md");

        // Fuzz the frontmatter extraction
        // This shouldn't panic regardless of input
        let _ = calvin::parser::extract_frontmatter(content, fake_path);

        // If content looks like YAML, also fuzz the parse_frontmatter
        if !content.is_empty() {
            let _ = calvin::parse_frontmatter(content, fake_path);
        }
    }
});
