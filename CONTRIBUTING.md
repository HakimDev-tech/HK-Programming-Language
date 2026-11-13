# Contributing

HK is an early-stage language project. Syntax and semantic changes should be discussed in the specification before implementation.

1. Read the language specification and architecture.
2. Keep changes focused.
3. Add tests for observable behavior.
4. Update specification and examples when semantics change.
5. Run cargo fmt, cargo test, and cargo clippy.

Rules:
- No implicit type coercions.
- Conditions must be Boolean.
- Preserve source locations.
- Do not use panics for invalid HK programs.
- Avoid unsafe Rust without a measured requirement and review.
- Do not describe roadmap items as implemented.

Pull requests should describe semantic changes, compatibility impact, tests, and known limitations.
