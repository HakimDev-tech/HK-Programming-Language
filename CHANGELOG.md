# Changelog

## HK 0.4.0 — Unreleased

- Add an AST-to-bytecode compiler with source-aware instructions and gradual-typing provenance.
- Add mandatory bytecode verification for control flow, operand stack, lexical scopes, captures, and prototype references.
- Add a stack-based VM with lexical upvalues, mutable shared binding cells, bounded call depth, and V0.3-compatible weak closure lifetime.
- Route hk::run through verified bytecode on the V0.4 implementation branch.
- Compile dependency-ordered filesystem modules into one verified bytecode program and execute hk::run_path through the VM.
- Preserve module isolation, named import/export aliases, live bindings, std:math, and source-aware runtime diagnostics.
- Add source and module differential tests against the retained tree-walking reference engine.
- Add a reproducible end-to-end VM benchmark harness and record an initial release-build baseline.
- Complete local 12-second fuzz smoke campaigns for source execution, module resolution, and malformed-bytecode verification; longer CI fuzz campaigns and final cross-platform CI on the exact release candidate remain required.
- Same-host V0.3/V0.4 end-to-end benchmark comparison completed; the current V0.4 candidate is 13.9–42.3% slower on the measured workloads. This is documented for explicit release acceptance or optimization; no performance-improvement claim is made.
- V0.4.0 remains unreleased pending final CI/fuzz review, performance-regression acceptance or remediation, and release approval.

## HK 0.3.0 — 2026-11-15

- Add top-level `import { name as alias } from "./module.hk";` and `export` declarations.
- Add filesystem-backed `hk::run_path` and route the CLI file runner through the module loader.
- Resolve imports relative to the importer, canonicalize module identities, deduplicate dependencies, and initialize dependencies before importers.
- Validate the reachable graph and export links before execution; reject cycles and duplicate module bindings.
- Isolate module root environments and preserve live, read-only imported aliases and defining environments for exported functions.
- Preserve defining-module source context for runtime errors raised inside imported functions.
- Add stable diagnostics for module resolution, export linking, cycles, imported-binding reassignment, and standard-library contract failures.
- Bound module loading to 128 dependency levels, 1,024 files, 1 MiB per file, and 16 MiB aggregate source.
- Add `std:math` with `abs`, `ceil`, `floor`, `round`, `sqrt`, `min`, and `max`.
- Add module integration tests, a runnable module example, and a dedicated module-specifier fuzz target.
- Final release-candidate validation passed on Linux, Windows, and macOS: formatting, workspace tests, Clippy with warnings denied, rustdoc, parser/interpreter fuzzing, and module-resolver fuzzing. The implementation is merged into `main`; Git tag and GitHub Release publication remain pending.

## HK 0.2.0 — 2026-11-15

- Classify runtime type diagnostics by typed-expression provenance rather than a file-wide annotation flag, preserving legacy `E3000` errors in unrelated dynamic regions while retaining `E3008` for typed guards.
- Set the distributable `hk` crate version to `0.2.0` and add automated execution tests for all five documented examples.
- Merge the validated HK 0.2.0 gradual-typing integration into `main`.
- Preserve 0.1.x dynamic behavior in unannotated regions of mixed typed/dynamic programs while retaining static checks at declared boundaries.
- Add optional annotations for variables and named-function parameters and return values using `number`, `string`, `bool`, `null`, `array`, and `any`.
- Add static type checking before runtime execution, including variable initialization/assignment, known function arguments/returns, operator operands, conditions, and conservative return-path checks.
- Enforce runtime contracts on typed bindings, function parameters, and function returns; guard dynamic values at concrete typed boundaries.
- Preserve dynamic behavior for unannotated bindings and dynamic function aliases.
- Keep arrays heterogeneous; contracts apply to bindings, not array elements or shared array objects.
- Improve runtime diagnostic provenance for parameter, return, operator, condition, array receiver, and index boundary failures.
- Align checker scope handling with runtime lexical scopes: explicit blocks create scopes, while `if` and `while` do not add synthetic scopes.
- Analyze `for` clauses in runtime order: initializer, condition, body, then increment.
- Reject calls through statically known concrete non-function values with E3010; calls through `any` retain runtime callability checks.
- Add a comprehensive gradual-typing regression matrix and typed fuzz corpus while preserving the 0.1.x compatibility suite.

## Unreleased — HK 0.1.x hardening

- Split the interpreter into lexer, AST, parser, runtime, and diagnostics modules.
- Add source excerpts and caret markers to lexical, syntax, and runtime diagnostics.
- Bound recursive parser, evaluator, statement, function-call, and array-display depth.
- Make lexical environment traversal and array-reference graph checks iterative.
- Expand malformed-input, deep-nesting, and generated-source regression coverage.
- Enforce rustfmt checks and cross-platform CI on Linux, Windows, and macOS.

## Unreleased — HK 0.1.0 baseline

- Establish HK 0.1.0 language specification and architecture.
- Add initial Rust lexer, parser, interpreter, runtime, CLI, examples, and tests.
