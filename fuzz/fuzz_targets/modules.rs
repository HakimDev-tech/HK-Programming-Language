#![no_main]

use libfuzzer_sys::fuzz_target;
use std::{
    fs,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_CASE: AtomicU64 = AtomicU64::new(0);

fuzz_target!(|input: &[u8]| {
    let specifier = String::from_utf8_lossy(input);
    let quoted_specifier = format!("{specifier:?}");
    let source = format!("import {{ value }} from {quoted_specifier};");
    let case = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "hk-module-fuzz-{}-{case}",
        std::process::id()
    ));
    if fs::create_dir_all(&root).is_err() {
        return;
    }
    let entry = root.join("main.hk");
    if fs::write(&entry, source).is_ok() {
        let _ = hk::run_path(&entry);
    }
    let _ = fs::remove_dir_all(root);
});
