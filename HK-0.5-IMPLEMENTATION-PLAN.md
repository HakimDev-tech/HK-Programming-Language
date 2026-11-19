# HK 0.5.0 — Implementation Plan

**Current phase:** M1.1 — front-end model and grammar (in progress; isolated lexer/type-expression model plus initial struct/enum AST/parser slice in PR #21; full grammar and executable integration outstanding).
**Normative contract:** HK-0.5-SPEC.md.
**Base:** HK 0.4.0 merge commit c47a06cec1120edd33114cce1857c40f10b7f333.
**Release status:** not implemented; not released.

## Delivery model

Implement in dependency order. Each milestone is a focused reviewable change and must pass the existing suite. The v0.4 VM is the compatibility baseline. No milestone is complete from code inspection alone; it requires automated evidence.

| ID | Scope | Mandatory exit evidence |
|---|---|---|
| M1.1 | Tokens, type-expression AST, declarations, impl AST, patterns, parser | Grammar tests for valid/invalid/ambiguous input and source spans. Partial foundation: token vocabulary, spans, TypeExpr/GenericBound, initial type/bound parsers; full grammar remains open. |
| M1.2 | Whole-graph declaration collection and symbol tables | Duplicate/unknown declaration tests; deterministic resolution; no initializers executed |
| M2.1 | Nominal types, generic type resolution, field/constructor checking | Positive/negative checker matrix and dynamic boundary tests |
| M2.2 | Trait conformance, orphan/coherence, generic inference/bounds | Multi-module coherence and inference ambiguity/limit tests |
| M2.3 | Match exhaustiveness and pattern binding scopes | Exhaustive/non-exhaustive/duplicate/invalid pattern tests |
| M3.1 | Safe nominal descriptors and struct/enum runtime values | Identity, aliasing, mutation, payload, cycle/allocation-limit tests |
| M3.2 | Methods and runtime lookup for any | Receiver aliasing, defining-environment and dynamic lookup tests |
| M4.1 | Bytecode descriptors/constants/opcodes | Compiler golden tests and format invariants |
| M4.2 | Mandatory verifier extensions | Adversarial forged-ID/layout/variant/method/specialization/stack tests |
| M5 | VM and source execution | VM tests and reference-engine parity |
| M6 | Module type exports/imports and whole-graph checks | Cross-module types/traits/impls and pre-execution failure tests |
| M7 | Hardening | Fuzzing, resource limits, diagnostics, benchmarks, cross-platform CI |
| M8 | Release readiness | Docs/API review, exact-head CI/fuzz, merge, tag and GitHub Release |

## M1.1 checklist

**Progress note (2026-11-19):** PR #21 contains the isolated lexer/type-expression foundation and initial struct/enum declaration AST/parser (generic params/bounds, typed fields, unit/tuple variants, duplicate diagnostics and source spans). Generic lists now reject duplicate parameters/bounds and are bounded at 256 each. Dedicated fuzz target `syntax_v05` exercises lexer/type-expression/nominal parser inputs. Code head `6db4fd9b04ca211226bc16f578eab9af65438479` passes CI on Linux/macOS/Windows (format, workspace tests, Clippy with warnings denied, rustdoc); fuzz campaign #351 is pending. These validate this isolated slice only; M1.1 remains incomplete and no v0.5 feature is integrated into executable source compilation.

- Add keywords and punctuation without changing existing tokenization.
- Represent recursive type expressions: primitive, nominal path, generic application and type parameter.
- Preserve spans for declarations, fields, variants, methods, bounds, type arguments and patterns.
- Parse top-level struct/enum/trait declarations and inherent/trait impl blocks.
- Parse generic functions and turbofish calls.
- Parse struct literals and field assignment without confusing block syntax.
- Parse enum construction and exhaustive statement-level match arms.
- Enforce module-top-level and import/export restrictions consistently.
- Never execute code or mutate runtime environments during declaration collection.

## Mandatory invariants

1. Collect all reachable declarations before checking/executing module bodies.
2. Resolve symbols deterministically, independent of hash-map iteration order.
3. Generic substitution is capture-avoiding and bounded by the normative limits.
4. Each concrete instantiation has one stable program-local descriptor/specialization.
5. Runtime nominal checks compare descriptor identity, not display names.
6. Every VM path accepts only a fully verified program.
7. New AST variants are exhaustively handled by checker, compiler, reference runtime, module loader, analysis and tests.
8. Malformed source/bytecode cannot reach user-triggerable panic/unwrap paths.
9. Existing unannotated 0.4 programs retain behavior.
10. No milestone is complete without CI and regression evidence.

## Test matrix

- Lexer/parser: every new keyword/operator, comments/whitespace, nested generic delimiters, turbofish, duplicate names, missing delimiters and source positions.
- Checker: nominal identity, invariance, fields, receivers, trait conformance, orphan/coherence, inference, bounds and any boundaries.
- Match: exhaustive, wildcard, duplicate, missing, wrong enum, wrong arity, shadowing and scope.
- Runtime: aliasing/mutation, enum integrity, identity equality, cycles, resource limits, method environment/source identity.
- Verifier: invalid descriptor IDs, generic metadata, field offsets, enum tags/arity, method targets, match tables, stack/scope corruption.
- Modules: type imports/exports, cross-module impls, orphan rule, duplicate impl detection before side effects, deterministic linking.
- Differential: old semantics and all new features once reference engine supports them.
- Fuzz: parser, type graph, substitution/instantiation, coherence graph, module linker and verifier.
- Benchmarks: parsing, checking, monomorphization, verification, execution, bytecode size and peak memory versus 0.4.

## Release blockers

Any unresolved normative semantic rule; any unverified descriptor/opcode; panic or unbounded recursion from user input; 0.4 regression without approval; reference/VM mismatch; nondeterministic inference/specialization/trait/module resolution; missing fuzz, CI, documentation or benchmark evidence.

## Release checklist

- [ ] M1–M7 exit gates satisfied.
- [ ] Workspace tests and Clippy pass.
- [ ] Linux/macOS/Windows CI passes on exact release commit.
- [ ] Fuzz campaigns finish without findings.
- [ ] Benchmarks and known limitations published.
- [ ] README, language spec, architecture, changelog and examples audited.
- [ ] PR reviewed and merged.
- [ ] Annotated v0.5.0 tag targets the exact merge commit.
- [ ] GitHub Release published and verified.


## Continuation progress — 2026-11-19

New staged parser work adds trait declarations and impl blocks with method signatures, generic impl headers, method body span capture, duplicate checks, and bounded body nesting. The syntax fuzz target exercises trait/impl parsing. Commit `eefd7dc479375a861cd799d0bf9ec25fbb6d2e44` is awaiting CI. This does not complete M1.1: bodies remain opaque token spans and no v0.5 feature is integrated into the executable compiler.

**Latest verified update (2026-11-19):** implementation head `d9270d6c9d6be68439aa1ddee5295325897dcec3`; CI #842 passed Linux/macOS/Windows with workspace tests, formatting, Clippy and rustdoc. The trait/impl parser slice now enforces a first `self` receiver and captures bodies opaquely. Fuzz #371 is in progress. Earlier CI failures were fixed; do not treat those attempts as passing. No compiler integration yet; M1.1 incomplete, M1.2–M8 remain.