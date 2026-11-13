# Fuzzing HK

HK has two layers of input robustness coverage:

1. Deterministic regression tests in the normal workspace suite, including malformed inputs, generated source strings, deep nesting, and panic checks.
2. A dedicated cargo-fuzz target that feeds arbitrary byte sequences to the public `hk::run` entry point. Non-UTF-8 inputs are ignored because HK source files are UTF-8.

## Local usage

Install cargo-fuzz with a nightly Rust toolchain, then run:

~~~bash
cargo install cargo-fuzz --locked
cargo fuzz build run
cargo fuzz run run -- -max_total_time=120 -max_len=4096
~~~

Crashes and minimized inputs should be retained as regression fixtures and converted into deterministic tests before a fix is considered complete.

## Continuous fuzzing

The GitHub Actions workflow builds the fuzz target and runs a bounded 45-second fuzz session on relevant pushes and pull requests. A weekly scheduled run provides additional coverage.

The harness treats successful execution and structured HK diagnostics as valid outcomes. It is intended to detect panics and pathological parser/runtime behavior, not to validate the semantic correctness of arbitrary programs.
