# Fuzzing HK

HK currently has deterministic malformed-input and generated-input regression tests in the normal workspace suite. For longer-running coverage, run the test suite repeatedly with different source corpora and preserve every discovered crash as a regression test.

Recommended local commands:

~~~bash
cargo test --workspace
cargo test -p hk generated_source_inputs_never_panic
~~~

A dedicated cargo-fuzz target is intentionally deferred until the interpreter API and AST boundaries are stable enough to avoid maintaining a duplicate fuzz harness. The source entry point to fuzz is hk::run(source_name, source); fuzzers must treat both successful execution and HkError as valid outcomes and flag panics, hangs, or unbounded resource growth.
