# HK 0.2.0 — Implementation Plan

**Status:** Alpha.1 syntax, alpha.2 static checking, and alpha.3 runtime contracts are implemented on a feature branch; alpha.4 is a bounded stabilization pass, not a new feature phase.

The normative gradual-typing contract is in [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md). This plan must follow that specification. A semantic change requires updating the specification and tests before implementation.

## Release objective

Introduce optional annotations for variables and named-function parameters/returns, a static checker, and runtime guards at dynamic-to-typed boundaries, while preserving unannotated 0.1.x behavior.

## Phase 0 — Design baseline

- [x] Select gradual typing as the 0.2.0 model
- [x] Define source-level types: number, string, bool, null, array, any
- [x] Define annotation grammar for variables and named functions
- [x] Define static compatibility and dynamic boundary rules
- [x] Define array behavior and return-path policy
- [x] Define architecture, diagnostics, acceptance criteria, and non-goals
- [x] Review and clarify dynamic operator guards, short-circuit evaluation, function aliases, sequential name resolution, assignment-expression typing, array aliasing, and null/any return behavior

## Phase 1 — Frontend syntax (alpha.1)

- [x] Add colon and arrow tokens with regression coverage
- [x] Extend AST with optional type annotations
- [x] Parse variable annotations
- [x] Parse parameter annotations
- [x] Parse function return annotations
- [x] Reject unknown type names and malformed annotations with source diagnostics
- [x] Add parser tests for valid and invalid syntax
- [x] Confirm all existing unannotated syntax remains unchanged

**Alpha.1 gate:** satisfied — annotated source parses into an AST with source locations.

**Alpha.2 gate:** satisfied — static checking runs before execution and rejects known mismatches; wholly unannotated programs retain the dynamic 0.1.x path.

**Alpha.3 gate:** satisfied in implementation — valid annotated programs execute; dynamic values are checked at typed binding and function boundaries.

## Phase 2 — Static type system (alpha.2)

- [x] Add the source-level type representation and compatibility helpers
- [x] Implement lexical type scopes consistent with runtime scopes
- [x] Check variable initializers and assignments
- [x] Check unary and binary operators
- [x] Check Boolean conditions and preserve short-circuit behavior
- [x] Track named function signatures, including self-recursion
- [x] Check statically known function arguments and return expressions
- [x] Implement conservative typed-function return-path analysis
- [x] Add stable E3001–E3009 diagnostics
- [x] Ensure static errors occur before any runtime output or side effect

**Exit gate:** all statically provable type violations are rejected before execution; dynamic expressions remain dynamic.

## Phase 3 — Runtime contracts (alpha.3)

- [x] Store declared type metadata on runtime bindings
- [x] Centralize initialization and assignment contract checks
- [x] Store parameter and return contract metadata on functions
- [x] Guard dynamic arguments before function body execution
- [x] Guard dynamic return values before they escape the function
- [x] Guard dynamic values used in concrete typed contexts
- [x] Ensure every violation returns an HK diagnostic, never a Rust panic
- [x] Test aliasing, shadowing, recursion, and all mutation paths

**Exit gate:** implementation is present on the alpha.3 feature branch; stabilization verifies the acceptance matrix and cross-platform CI before merge.

## Phase 4 — Stabilization (alpha.4)

- [x] Audit runtime diagnostic classification for mixed typed/dynamic programs; preserve E3000 for non-contract operator/call failures
- [x] Add regression tests for dynamic  overload failures and non-callable dynamic values
- [x] Add a dedicated seed corpus for variable contracts, indirect function contracts, and short-circuit behavior
- [x] Preserve the existing 0.1.x regression tests while adding gradual-typing coverage
- [x] Add positive, negative, and dynamic-boundary acceptance matrices in `compiler/crates/hk/tests/gradual_typing_matrix.rs`
- [x] Extend fuzzing with annotation-heavy seeds and complete a successful corpus run on the preceding stabilization head
- [ ] Complete the dedicated fuzz run on the final alpha.4 head
- [x] Review the normative specification, diagnostics, README, roadmap, and executable example
- [x] Run fmt, tests, Clippy with warnings denied, and rustdoc with warnings denied on the final alpha.4 head
- [x] Verify Linux, Windows, and macOS CI on the final alpha.4 head
- [x] Record known limitations and the alpha.4 stabilization changes in the specification and changelog

**Exit gate:** every acceptance criterion in HK-TYPING-SPEC.md passes on the same final commit; no alpha.4 completion claim before final CI and review are green.

## Release acceptance

A release candidate is not a release. Publish 0.2.0 only after all mandatory acceptance criteria pass and the implementation, specification, examples, and changelog agree.

## Explicitly deferred

Generics, unions, optional types, typed array elements, flow-sensitive narrowing, global type inference, function types, anonymous functions, closures, modules, bytecode, VM, native compilation, WASM, and FFI are outside this release.
