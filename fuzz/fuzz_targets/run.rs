#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    if let Ok(source) = std::str::from_utf8(input) {
        let _ = hk::run("<fuzz-input>", source);
    }
});
