# Changelog

## Unreleased — HK 0.3.0 (implementation in progress)

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
- Cross-platform CI, full regression validation, and the final release audit remain pending.

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
