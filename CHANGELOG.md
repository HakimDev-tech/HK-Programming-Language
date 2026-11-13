## Unreleased — HK 0.2.0-alpha.4

- Preserve legacy E3000 diagnostics for dynamic `+` overload failures and non-callable values, rather than misclassifying them as runtime contract violations (E3008).
- Add regression coverage for diagnostic classification in mixed typed/dynamic programs.
- Add positive, negative, and dynamic-boundary acceptance matrices for gradual typing.
- Seed the fuzz corpus with typed variable/function contract and short-circuit cases; a successful annotation-heavy run completed on the preceding stabilization head, and the final-head run is pending.
- Keep alpha.4 scoped to stabilization; no new language feature is introduced.


# Changelog

## Unreleased — HK 0.2.0-alpha.3

- Execute programs containing valid gradual type annotations after the alpha.2 static checker succeeds.
- Store declared type contracts on individual runtime bindings, preserving dynamic unannotated bindings and independent array aliases.
- Enforce annotated variable initialization and reassignment before writes become observable.
- Enforce annotated function parameter contracts before entering the function body, including calls through dynamic aliases.
- Enforce annotated return contracts on explicit returns and implicit fallthrough (`null` and `any`).
- Add E3008 runtime contract diagnostics and regression tests for dynamic-to-typed boundaries, aliasing, and array mutation.
- Keep typed array bindings heterogeneous: contracts constrain the binding, not array elements.


## Unreleased — HK 0.2.0-alpha.2

- Add an internal type representation and compatibility relation.
- Add lexical type scopes and static checks for annotated variable initialization, assignment, direct function calls, return values, operators, conditions, and conservative return-path completeness.
- Run static checking before the alpha execution gate; wholly unannotated programs remain on the dynamic 0.1.x path.
- Add regression tests for static mismatches, return fallthrough, function shadowing, mutable parameters, and unannotated compatibility.
- Typed programs were gated until alpha.3 runtime contract enforcement.

## Unreleased — HK 0.2.0-alpha.1

- Recognize `:` and `->` in the lexer.
- Parse optional variable, parameter, and function return annotations for `number`, `string`, `bool`, `null`, `array`, and `any`.
- Preserve annotations in dedicated AST nodes with source locations.
- Initially gate execution of annotated programs with E2004; alpha.2 added static checking and alpha.3 removed this gate after runtime contracts were implemented.
- Add regression tests for supported type names, typed declarations/functions, unknown types, and nested annotations.

## Unreleased — HK 0.2.0 design

- Select gradual typing as the 0.2.0 model.
- Specify optional annotations for variables and named-function parameters/returns.
- Define static checking, operator-specific runtime guards, dynamic function-call boundaries, heterogeneous array behavior, assignment-expression typing, and conservative return-path analysis.
- Clarify short-circuit guard timing, sequential name resolution, array binding contracts, and null/any return fallthrough.
- Add the normative design baseline in HK-TYPING-SPEC.md and the phased implementation plan.
- Gradual typing semantics are specified in HK-TYPING-SPEC.md.

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
