# HK 0.2.0 — Implementation Plan

**Status:** Alpha.1 syntax, alpha.2 static checking, and alpha.3 runtime contracts are implemented on a feature branch; cross-platform CI and review remain release gates.

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

**Exit gate:** implemented on the alpha.3 feature branch; verify via runtime contract test matrix and cross-platform CI before merge.

## Phase 4 — Stabilization and release candidate

- [ ] Run the complete 0.1.x regression suite unchanged
- [ ] Add positive, negative, and dynamic-boundary test matrices
- [ ] Add malformed-input and generated-input panic tests
- [ ] Extend fuzzing to parser annotations and type-checker paths
- [ ] Review diagnostics, documentation, and examples
- [ ] Run fmt, tests, Clippy with warnings denied, and rustdoc with warnings denied
- [ ] Verify Linux, Windows, and macOS CI
- [ ] Record known limitations and release notes

**Exit gate:** every acceptance criterion in HK-TYPING-SPEC.md passes.

## Release acceptance

A release candidate is not a release. Publish 0.2.0 only after all mandatory acceptance criteria pass and the implementation, specification, examples, and changelog agree.

## Explicitly deferred

Generics, unions, optional types, typed array elements, flow-sensitive narrowing, global inference, function types, anonymous functions, closures, modules, bytecode, VM, native compilation, WASM, and FFI are outside this release.
