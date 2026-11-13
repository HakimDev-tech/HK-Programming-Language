# HK 0.2.0 — Finalization Plan

**Status:** The gradual-typing implementation is complete on the integration candidate branch. It includes annotation syntax, static checking, runtime contracts, precise boundary diagnostics, and checker/runtime scope conformance. Finalization consolidates the implementation, closes semantic gaps found by audit, and verifies one exact commit before release consideration.

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
- [ ] Review panic safety, depth limits, and malformed-source behavior.

### 3. Documentation and release consistency

- [x] Consolidate incremental engineering history into one HK 0.2.0 changelog entry.
- [x] Align architecture documentation with implemented module boundaries.
- [x] Align README and roadmap with the actual branch and release status.
- [x] Consolidate scope and loop-analysis rules into the normative typing specification.
- [ ] Verify all examples against the final interpreter.
- [ ] Verify every public claim against implementation and tests.

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

Passing these gates does not authorize publication or merging. HK 0.2.0 remains unreleased until the integration is reviewed and explicitly approved.

## Deferred beyond 0.2.0

Generics, unions, optional types, typed array elements, flow-sensitive narrowing, global type inference, source-level function types, anonymous functions, closures, modules, bytecode, VM, native compilation, WASM, FFI, package management, LSP, formatter, and debugger remain deferred.
