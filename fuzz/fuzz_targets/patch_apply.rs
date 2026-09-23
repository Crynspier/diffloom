#![no_main]

use diffloom_core::{patch_apply, patch_from_text, MatchOptions};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    let mut parts = input.splitn(2, '\0');
    let patch = parts.next().unwrap_or_default();
    let text = parts.next().unwrap_or_default();
    if let Ok(parsed) = patch_from_text(patch) {
        let _ = patch_apply(&parsed, text, MatchOptions::default());
    }
});
