# Changelog

## HK 0.4.0 — Merged; publication metadata pending

- Add an AST-to-bytecode compiler with source-aware instructions and gradual-typing provenance.
- Add mandatory bytecode verification for control flow, operand stack, lexical scopes, captures, and prototype references.
- Add a stack-based VM with lexical upvalues, mutable shared binding cells, bounded call depth, and 0.3-compatible weak closure lifetime.
- Route public `hk::run` and `hk::run_path` execution through verified bytecode.
- Compile dependency-ordered filesystem modules into one verified bytecode program.
- Preserve module isolation, named import/export aliases, live bindings, `std:math`, and source-aware runtime diagnostics.
- Add source and module differential tests against the retained tree-walking reference engine.
- Add reproducible end-to-end VM benchmarks and document the measured 5.3–29.4% regressions on the measured workloads as an accepted architectural trade-off, not a performance improvement.
- The final candidate passed cross-platform CI and dedicated fuzz campaigns. Post-merge CI on commit `c47a06c` passed on Ubuntu, macOS, and Windows.
- Git tag and GitHub Release publication are separate operations; merge and CI success do not imply that publication metadata exists.

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

## HK 0.2.0 — 2026-11-15

- Add optional annotations for variables and named-function parameters and return values using `number`, `string`, `bool`, `null`, `array`, and `any`.
- Add static type checking before runtime execution and runtime contracts at typed boundaries.
- Preserve dynamic behavior for unannotated bindings and dynamic function aliases.
- Keep arrays heterogeneous; contracts apply to bindings, not array elements or shared array objects.
- Add a comprehensive gradual-typing regression matrix while preserving the 0.1.x compatibility suite.

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
