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
    let main_source = "import { value } from \"./dep.hk\"; print(value);";
    let dependency_source = format!(
        "import {{ abs as imported_abs }} from {quoted_specifier}; export const value = 1;"
    );
    let case = NEXT_CASE.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "hk-module-fuzz-{}-{case}",
        std::process::id()
    ));
    if fs::create_dir_all(&root).is_err() {
        return;
    }

    let files = [
        ("main.hk", main_source),
        ("dep.hk", dependency_source.as_str()),
        ("dep2.hk", "export const abs = 1;"),
    ];
    for (name, source) in files {
        if fs::write(root.join(name), source).is_err() {
            let _ = fs::remove_dir_all(&root);
            return;
        }
    }

    let _ = hk::run_path(root.join("main.hk"));
    let _ = fs::remove_dir_all(root);
});
