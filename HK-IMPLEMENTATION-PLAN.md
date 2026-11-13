# HK 0.2.0 — Implementation Plan

**Status:** Alpha.1–Alpha.3 are implemented; Alpha.4 stabilization is validated. Alpha.5 boundary-conformance implementation, acceptance coverage, cross-platform CI, and final fuzz validation are complete on its feature branch. PR #10 remains open and unmerged. Alpha.6 is specified as a scope and loop-analysis conformance increment; implementation has not started.

The normative gradual-typing contract is in [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md). Phase-specific acceptance contracts are documented in [HK-ALPHA5-SPEC.md](HK-ALPHA5-SPEC.md) and [HK-ALPHA6-SPEC.md](HK-ALPHA6-SPEC.md). A semantic change requires updating the specification and tests before implementation.

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
- [x] Add regression tests for dynamic overload failures and non-callable dynamic values
- [x] Add a dedicated seed corpus for variable contracts, indirect function contracts, and short-circuit behavior
- [x] Preserve the existing 0.1.x regression tests while adding gradual-typing coverage
- [x] Add positive, negative, and dynamic-boundary acceptance matrices in `compiler/crates/hk/tests/gradual_typing_matrix.rs`
- [x] Extend fuzzing with annotation-heavy seeds and complete a successful corpus run on the preceding stabilization head
- [x] Complete the dedicated fuzz run on the final alpha.4 head
- [x] Review the normative specification, diagnostics, README, roadmap, and executable example
- [x] Run fmt, tests, Clippy with warnings denied, and rustdoc with warnings denied on the final alpha.4 head
- [x] Verify Linux, Windows, and macOS CI on the final alpha.4 head
- [x] Record known limitations and the alpha.4 stabilization changes in the specification and changelog

**Exit gate:** every acceptance criterion in HK-TYPING-SPEC.md passes on the same final commit; no alpha.4 completion claim before final CI and review are green.

## Phase 5 — Boundary conformance and diagnostic provenance (alpha.5)

- [x] Specify runtime boundary invariants and diagnostic source-location requirements in [HK-ALPHA5-SPEC.md](HK-ALPHA5-SPEC.md)
- [x] Report parameter contract failures at the offending argument, including calls through aliases
- [x] Report explicit return contract failures at the returned expression
- [x] Report unary/binary/logical guard failures at the offending operand
- [x] Report array receiver and index guard failures at their respective expressions
- [x] Add regression tests for left/right operands, logical short-circuit boundaries, array receivers/indexes, and function alias boundaries
- [x] Verify function/variable shadowing, independent typed shadowed bindings, and mutual recursion behavior
- [x] Verify failed parameter validation prevents function-body side effects and evaluates argument expressions once
- [x] Add Alpha.5-specific fuzz seeds for argument, return, and dynamic operator boundaries
- [x] Complete boundary coverage for initialization/assignment, direct/aliased parameters, explicit/implicit returns, operator guards, conditions, array read/write receivers and indexes, and short-circuit branches
- [x] Audit static/runtime lexical shadowing, function aliases, direct recursion, and dynamic forward/mutual recursion behavior
- [x] Run final fmt, workspace tests, Clippy, rustdoc, and Linux/Windows/macOS CI on the final candidate head
- [x] Complete the final fuzz session on the final candidate head
- [x] Review the normative typing spec, Alpha.5 spec, README, roadmap, changelog, implementation plan, and gradual-typing example against the current behavior

**Exit gate:** all requirements in [HK-ALPHA5-SPEC.md](HK-ALPHA5-SPEC.md) pass on one exact final commit. Alpha.5 is not a release candidate.

## Phase 6 — Scope and loop-analysis conformance (alpha.6)

- [x] Audit checker scope creation against runtime environment creation
- [x] Verify declaration grammar: branch and loop declarations require explicit blocks
- [x] Remove synthetic checker scopes around `if` and `while`; preserve explicit `S::Block` scopes
- [x] Analyze `if` branches independently through normal block-scope entry/exit
- [x] Analyze `for` clauses in runtime order: initializer → condition → body → increment
- [x] Preserve loop scope across initializer, condition, body, and increment; discard it after the loop
- [x] Add focused regressions for branch isolation, outer contracts, block lifetime, loop diagnostic order, and loop-scope lifetime
- [x] Preserve all V0.1.x and Alpha.1–Alpha.5 tests unchanged
- [ ] Run fmt, workspace tests, Clippy, rustdoc, Linux/Windows/macOS CI, and fuzzing on one exact final commit
- [ ] Review all normative documentation against the implementation

**Exit gate:** all requirements in [HK-ALPHA6-SPEC.md](HK-ALPHA6-SPEC.md) pass on one exact final commit. Alpha.6 is a checker-conformance increment, not a release candidate.

## Release acceptance

A release candidate is not a release. Publish 0.2.0 only after all mandatory acceptance criteria pass and the implementation, specification, examples, and changelog agree.

## Explicitly deferred

Generics, unions, optional types, typed array elements, flow-sensitive narrowing, global type inference, function types, anonymous functions, closures, modules, bytecode, VM, native compilation, WASM, and FFI are outside this release.
