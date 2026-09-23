#![no_main]

use diffloom_core::DiffEngine;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };
    let mut parts = input.splitn(2, '\0');
    let old = parts.next().unwrap_or_default();
    let new = parts.next().unwrap_or_default();
    let _ = DiffEngine::default().diff_with_status(old, new);
});
