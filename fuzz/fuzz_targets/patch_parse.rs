#![no_main]

use diffloom_core::patch_from_text;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(input) = std::str::from_utf8(data) {
        let _ = patch_from_text(input);
    }
});
