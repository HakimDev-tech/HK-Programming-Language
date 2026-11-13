# Changelog

## Unreleased — HK 0.2.0 design

- Select gradual typing as the 0.2.0 model.
- Specify optional annotations for variables and named-function parameters/returns.
- Define static checking, operator-specific runtime guards, dynamic function-call boundaries, heterogeneous array behavior, assignment-expression typing, and conservative return-path analysis.
- Clarify short-circuit guard timing, sequential name resolution, array binding contracts, and null/any return fallthrough.
- Add the normative design baseline in HK-TYPING-SPEC.md and the phased implementation plan.
- No 0.2.0 runtime or language feature is implemented by this documentation change.

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
