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

**Checkpoint 2026-11-19:** code head `bb547f7127499002c634dd30be8938c33af49eb9` adds the staged generic function parser to trait/impl parsing; CI #858 passed Linux/macOS/Windows (fmt, workspace tests, Clippy, rustdoc). Fuzz #378 is running. This remains isolated M1.1 partial work; no expression/body integration or semantic/compiler/runtime support is delivered.

**Latest checkpoint (2026-11-19):** added isolated struct-literal syntax parsing with field-value token spans, duplicate/empty checks, nested delimiter capture and cursor rollback. Fuzz target invokes it. Head `ffd1ea40eabdbf4c6b04abef2abfa2fb6afc9af3` CI #871 green across Linux/macOS/Windows. Fuzz #384 is pending/in progress. This remains partial M1.1; expression parsing and executable integration remain outstanding.

**Current exact checkpoint:** `4540a15e1e209c5c461e0b63a0e33a6f9efaedad` fixes fuzz-target cursor initialization and includes the struct literal parser. CI #878 passed on Linux/macOS/Windows (fmt, workspace tests, Clippy, rustdoc). Fuzz #387 is in progress. The parser is still isolated and does not parse or execute field expressions. M1.1 remains incomplete.

**Enum parser checkpoint (2026-11-19):** new standalone parser for fully-qualified enum constructors, unit variants and tuple payload spans; rejects empty/unbalanced payloads, bounds nesting and rolls back on error. CI #890 passed Linux/macOS/Windows. Fuzz run #387 completed all campaigns successfully on the earlier code slice; #391 with enum parser target is in progress. Still isolated M1.1; semantic checking/integration outstanding.

**Match syntax checkpoint (2026-11-19):** added parser for statement match, qualified enum variant patterns, binders, wildcard and arm blocks, with duplicate/wildcard ordering checks and transactional rollback. Head `d628663067e785cc53ce7658ee0cdccc57f311fa`, CI #904 passed all three OSes. Fuzz #398 is running (syntax target built, syntax campaign underway). Exhaustiveness and semantic analysis/compiler integration are not implemented; M1.1 remains partial.

**Fuzz evidence update:** run #398 completed successfully across all configured campaigns, including building/running the v0.5 syntax target with the match parser, module resolver and malformed-bytecode verifier. Code head `d628663067e785cc53ce7658ee0cdccc57f311fa` CI #904 passed all platforms. This is fuzz validation, not semantic/compiler integration; M1.1 incomplete.

**Execution update (2026-11-19):** A first integrated M1.2 namespace-collection pass has been added in `compiler/crates/hk/src/declaration_collector.rs`. It deterministically collects imports, module variables/functions and exports into a sorted symbol table; reserves the built-in `print`; and reports collisions before module import linking/initialization. The filesystem module loader now uses this pass to derive its export set. Unit tests cover sorted collection, exports, import/declaration collisions and reserved-name rejection. Commit `c608a45fe194961e544958da036dcd92ec07a088` triggered CI #924; status pending at checkpoint. This is a real integration slice, not completion of M1.2: nominal declarations, cross-module symbol/type resolution, forward-reference semantics and whole-graph trait/coherence collection are still missing. Local Rust execution is unavailable in this environment; CI is the verification gate.


**M1.2 progress update:** The collector is now wired into module loading and produces deterministic module symbols/export sets. The type checker also pre-collects root-level function signatures before checking bodies, enabling typed functions to reference later-declared typed functions and checking their argument contracts. Two direct checker tests cover forward-call success and argument-type failure. Code commits: `f7af3ba5847decbf2144426d1bf521a6c715e30c` (signature pass) and `88b055bca2f0429aa85cc1968dff05bfca703c2f` (tests). The earlier CI attempts failed on rustfmt only; the formatter diff was corrected in `ca76d5242e4d4581515c0d493fb1d88d3504acdc`. CI #930/#931 validate commit `f7af3ba...`; latest test commit CI is not yet confirmed. Forward declaration type resolution is implemented, but runtime function hoisting and the full v0.5 declaration graph are not.


**Latest integrated declaration/forward-call work (2026-11-19):** deterministic module namespace collection is integrated before import validation/initializers and in source/module compilation; root function signatures are precollected by the type checker; module-entry functions are hoisted by the compiler after allocating all function slots, enabling forward calls; reference interpreter hoisting and a parity test were added. CI #967 passed all three OSes on `a37bf96e514c192c8ec7934252739bbf50186338`. Head `fc8761efa188b491bc835ab00275e282260c2cf3` adds the reference-engine parity test and has CI #969/fuzz #428 pending. Formatting and test failures found in earlier attempts were fixed; only #967 is currently verified green for this integration slice. Nominal v0.5 declaration collection and all type/runtime/bytecode/VM/module feature integration remain outstanding.


## Verification checkpoint — 2026-11-19

- Latest code commit `02bfd9d78ab5fea8ab678a37cb7e68be00f93cb3` includes end-to-end typed forward-call test.
- CI #975 and #976: Linux/macOS/Windows green (format, tests, Clippy, rustdoc).
- Fuzz #431 green; preceding Fuzz #430 also green on checkpoint `fbdda2c0260c776fd243af05bd4f401c7d6c516c`.
- PR #21 is still Draft and unmerged. These checks validate existing integrated slices only, not the full v0.5 contract.


## Mixed top-level parser checkpoint — 2026-11-19

Implemented `parse_v05_top_level_declarations`: dispatches mixed struct/enum/trait/impl/function declaration streams to their specialized syntax parsers, retains source spans and rejects duplicate module-level names across kinds and unterminated declaration bodies. Unit tests cover mixed grammar/spans/duplicate names/malformed bodies. CI #998/#999 green on code commit `2ef96aa76db26c126d592496834bd7a816a30a83`. Fuzz target calls the mixed parser from commit `3affb2eaf4ea9414b270a542fe6895d3e5e0d5e9`; campaign #442 pending.

Limitations remain explicit: body grammar is opaque, module parser isn't wired into the production compiler, semantic resolution and runtime integration absent. M1.1 partial, not completed.


## Staged declaration analysis checkpoint — 2026-11-19

Added module-local nominal type-name resolution and generic arity validation (field, payload, function/method signatures, impl targets), generic parameter and trait-bound validation, plus coherence checks (orphan rule, inherent ownership, duplicate impls/methods, trait signature conformance/missing/extra methods, inherent/trait name conflicts). Added `analyze_v05_declaration_module` orchestration pipeline lex→mixed parser→type-name validation→coherence→sorted symbol index. Fuzz target exercises these APIs. CI #1032/#1033 green on `29bfbeda95b11acd8b2157d49d99b9d88a82fbd7`; fuzz #459 pending. Test commit `cb079b682de21ac7f410d08f5d65fc3a3844621b` pending.

This is staged frontend groundwork only: bodies are opaque and no production compiler integration, runtime, bytecode, verifier or VM nominal feature support is claimed.


## Coherence diagnostic correction — 2026-11-19

A CI assertion exposed an overly broad trait-method conflict diagnostic. The implementation now distinguishes inherent/trait name conflict from duplicate trait method names. CI #1045 is green on `d27da2199033fd09c3e9f46522435cdc6891393b` across Linux/macOS/Windows. Fuzz #459 completed successfully on the declaration analyzer checkpoint; campaign #465 on the updated validator remains running. Forward nominal-reference test commit `c015a78a82c6dba6aa245668cabed5763a2b8619` is awaiting validation.


## Generic coherence and lexer limits checkpoint — 2026-11-19

Implemented overlap-aware trait impl coherence (`Box<T>` vs concrete instantiations), overlap-aware inherent/trait method-set conflict detection, and 1 MiB source / 100,000-token v0.5 lexer bounds. Tests cover overlapping and disjoint generic impl patterns, generic inherent method collisions, and resource-limit diagnostics. CI #1078/#1079 green Linux/macOS/Windows on `f51fcaddc1bcec1614185af94258f92b2accb3ac`. Fuzz #481 is still running.

Staged semantics remain not integrated into executable compilation. M1.1 partial; M2 groundwork only; M3–M6 absent; M7 partial; M8 unreleased.
