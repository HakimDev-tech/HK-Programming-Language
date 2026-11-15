# HK 0.2.0 — Finalization Plan

**Status:** The release candidate `ff8be051dd46c8a6a7c9aca89e7b30592cc944c5` passed the full Linux/Windows/macOS CI matrix and the dedicated parser/interpreter fuzz run. Semantic review, regression coverage, and documentation/example verification are complete. PR #13 is ready for integration; the version remains unreleased until merge and release tagging.

The normative language contract is [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md). Internal engineering steps do not create separate Alpha versions.

## Release objective

Deliver HK 0.2.0 gradual typing while preserving 0.1.x dynamic behavior for unannotated code.

## Implemented capabilities

- Optional annotations on variable declarations, named-function parameters, and named-function returns.
- Source types: `number`, `string`, `bool`, `null`, `array`, and `any`.
- Static checking before runtime execution.
- Runtime contracts on typed bindings, parameters, and function returns.
- Runtime guards when dynamic values enter concrete typed contexts.
- Heterogeneous arrays with contracts attached to bindings, not array elements.
- Direct named-function signature checking and dynamic function-alias behavior.
- Conservative return-path analysis.
- Stable diagnostics for static type failures and runtime contract violations.
- Source-location provenance for dynamic boundary failures.
- Checker scope behavior aligned with explicit runtime lexical scopes.
- `for` clauses analyzed in runtime order: initializer, condition, body, increment.
- Cross-platform CI, regression matrix, and dedicated parser/interpreter fuzz workflow.

## Finalization work

### 1. Semantic audit

- [x] Compare normative typing rules with parser, checker, runtime, and integration tests.
- [x] Verify typed binding initialization and assignment contracts.
- [x] Verify parameter and return contracts, including aliases and fallthrough.
- [x] Verify operator guards, array receiver/index guards, and short-circuit timing.
- [x] Verify sequential function resolution, shadowing, recursion, and dynamic forward calls.
- [x] Verify explicit block scopes, branch isolation, loop scope lifetime, and `for` analysis order.
- [x] Identify and correct missing static rejection of calls through known concrete non-function values (E3010).
- [x] Review the final integration diff for unintended behavior changes.

### 2. Regression and robustness

- [x] Preserve the existing 0.1.x regression suite.
- [x] Maintain the gradual-typing acceptance matrix.
- [x] Add regression coverage for E3010 and preserve dynamic `any` call behavior.
- [x] Run the complete workspace test suite on the exact final integration commit.
- [x] Run parser/interpreter fuzzing on the exact final integration commit.
- [x] Re-audit panic safety, depth limits, malformed-source behavior, and diagnostic provenance after the final runtime correction.
- [x] Preserve dynamic checking for unrelated unannotated code in mixed typed/dynamic programs; enforce declared typed boundaries.

### 3. Documentation and release consistency

- [x] Consolidate incremental engineering history into one HK 0.2.0 changelog entry.
- [x] Align architecture documentation with implemented module boundaries.
- [x] Align README and roadmap with the actual branch and release status.
- [x] Consolidate scope and loop-analysis rules into the normative typing specification.
- [x] Execute every documented example against the final interpreter (automated integration smoke test).
- [x] Verify every public claim against implementation and tests.

### 4. Release gates

A 0.2.0 release candidate may be considered only after all of the following pass on the same exact commit:

- [x] `cargo fmt --all -- --check`
- [x] `cargo test --workspace`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo doc --workspace --no-deps`
- [x] Linux, Windows, and macOS CI
- [x] Dedicated fuzz workflow
- [x] Final semantic and documentation review
- [x] Explicit review of known limitations and compatibility guarantees

All technical gates are satisfied on the same exact candidate commit. HK 0.2.0 remains unreleased until PR #13 is merged and the `v0.2.0` release tag is created.

## Deferred beyond 0.2.0

Generics, unions, optional types, typed array elements, flow-sensitive narrowing, global type inference, source-level function types, anonymous functions, closures, modules, bytecode, VM, native compilation, WASM, FFI, package management, LSP, formatter, and debugger remain deferred.
