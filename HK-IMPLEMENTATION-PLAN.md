# HK 0.1.0 — Implementation Plan

## Delivered foundation

- [x] Rust workspace and package metadata
- [x] Source positions and diagnostics
- [x] Lexer for keywords, identifiers, numbers, strings, operators, punctuation, and comments
- [x] AST and parser for declarations, expressions, blocks, conditions, loops, functions, arrays, calls, indexing, and properties
- [x] Tagged runtime values and lexical environments
- [x] Mutable and constant bindings
- [x] Arithmetic, comparisons, equality, short-circuit logic
- [x] Conditions, while, and for
- [x] Named functions, recursion, return propagation
- [x] Arrays, indexing, mutation, length
- [x] print native function
- [x] CLI and initial integration tests
- [x] Specification, architecture, examples

## Remaining before a stable 0.1.0 release

- [ ] Run formatting, tests, and Clippy on supported platforms
- [ ] Add parser recovery and multi-error reporting
- [ ] Add tests for every diagnostic and source location
- [ ] Add fuzz/property-based testing for lexer and parser
- [ ] Profile recursion and memory behavior
- [ ] Publish release artifacts and installation instructions
- [ ] Audit the implementation against every normative rule in HK-LANGUAGE-SPEC.md

Unchecked items are not claimed as complete. The first commit is an implementation baseline, not a claim that release hardening is finished.
