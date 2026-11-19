# HK 0.5.0 — Independent implementation gap audit

**Audit ref:** feat/hk-v0.5.0-m1-front-end at `b79cb8f8ceb5364181c1aca8e20e9b0a6e7f3f3f` (2026-11-19)  
**Normative source:** `HK-0.5-SPEC.md`  
**Result:** 0.5.0 is not implemented. No milestone M1–M8 may be marked complete.

## Repository architecture inspected

Existing 0.4 pipeline is a single Rust crate in `compiler/crates/hk`: lexer/parser/AST, gradual type checker, module loader, reference runtime, bytecode compiler, verifier, and VM. Bytecode is in-memory/versioned. `hk::run` and `run_path` use verified VM execution; the reference engine remains for semantic comparison.

Existing source files: `ast.rs`, `lexer.rs`, `parser.rs`, `type_checker.rs`, `types.rs`, `compiler.rs`, `bytecode.rs`, `verifier.rs`, `vm.rs`, `runtime.rs`, `value.rs`, `environment.rs`, `module_loader.rs`, `diagnostics.rs`.

## Requirement-to-implementation map

| Requirement | Existing baseline | v0.5 gap | Required evidence |
|---|---|---|---|
| M1 lexer | v0.4 lexer; isolated `syntax_v05.rs` token model | Complete integrated tokens/operators/literals/comments/recovery/resource limits | lexer matrix, UTF-8 spans, malformed input |
| M1 AST | v0.4 expression/statement AST | Struct/enum/trait/impl/method/type/pattern/constructor/match AST and spans | AST coverage and exhaustive visitors |
| M1 parser | v0.4 parser | Full declaration/expression/pattern grammar, recovery, bounded nesting | grammar tests, malformed/adversarial tests |
| M1.2 declarations | Existing module binding validation | Whole graph nominal declaration, imports, forward references, duplicate checks, trait/coherence collection before init | graph fixtures, deterministic diagnostics, side-effect sentinel |
| M2.1 nominal types | Primitive gradual checker | Nominal identity, fields, construction/mutation, enum payloads, generic applications, invariance | positive/negative checker tests |
| M2.2 traits/generics | No v0.5 implementation | bounds, conformance, orphan/coherence, deterministic inference/substitution/monomorphization/limits | inference, bound, coherence and limit matrix |
| M2.3 match | No match AST/checker | exhaustive enum arms, binders/scopes, wildcard constraints, invalid/unreachable diagnostics | exhaustive/negative match tests |
| M3 runtime | Tagged v0.4 values, arrays/closures | RC heap structs, alias mutation, enum tags/payloads, stable nominal descriptors, cycle/depth bounded display | identity/alias/cycle/limit tests |
| M3.2 methods | Existing closure/function runtime | inherent/trait method resolution, defining environment, dynamic lookup for `any` | receiver/alias/environment tests |
| M4.1 bytecode | v1.0 in-memory bytecode, primitive opcodes | descriptors/layouts/tags/specializations/methods/match dispatch/opcodes | compiler golden/invariant tests |
| M4.2 verifier | Existing verifier checks 0.4 tables, control flow, stack/scope/captures | Verify every new metadata table/index/layout/target/type/tag/payload and limits; reject forged programs | adversarial forged-bytecode suite |
| M5 VM | Verified 0.4 VM | all nominal ops, dispatch, match, specialization, descriptors and mandatory verify | source-to-VM integration and differential tests |
| M6 modules | v0.4 file graph, imports/exports and init | nominal exports/imports, graph resolution/coherence before init, generic cross-module identity | cross-module and pre-init side-effect tests |
| M7 quality | Existing workspace tests, CI and fuzz targets for run/verifier/modules | feature-specific fuzzing, differential parity, benchmarks, limits, cross-platform full feature suite and audit | exact-head matrix, campaigns, benchmark report |
| M8 delivery | Current docs and draft PR | accurate docs/examples, changelog, release audit, merge, tag, GitHub release | exact commit evidence and release artifacts |

## Dependency-safe implementation sequence

1. Integrated AST and grammar while retaining 0.4 AST semantics.
2. Frontend integration and declaration graph/symbol identities.
3. Nominal/generic semantic model and checker.
4. Trait/coherence and exhaustive match analysis.
5. Runtime descriptors and heap values.
6. Compiler/bytecode metadata/opcodes.
7. Verifier extensions before enabling VM entry for any new op.
8. VM operations and source/reference behavior.
9. Module graph linking and initialization barriers.
10. Resource/security/differential/fuzz/benchmark hardening.
11. Docs and exact release-candidate validation.

## Current verified facts

- PR #21 is open and Draft; current head is `b79cb8f8ceb5364181c1aca8e20e9b0a6e7f3f3f`.
- The current head CI run #773 succeeded on Ubuntu, macOS, and Windows; fuzz run #337 succeeded.
- These runs validate the repository's current existing test/workflow scope, not v0.5 feature semantics.
- The current v0.5 syntax module is explicitly isolated and not connected to compiler execution. It now contains an initial struct/enum AST/parser slice and generic-list duplicate/limit checks; it still lacks the full grammar and semantic integration.
- No v0.5 runtime/compiler/verifier/VM/module feature is delivered. Current exact head `6db4fd9b04ca211226bc16f578eab9af65438479` passes the CI matrix; a dedicated `syntax_v05` fuzz target/campaign was added and is pending.
- No v0.5 release tag or release exists.

## Execution constraints encountered in this session

Repository edits can be made through the GitHub contents API and CI results can be inspected. A local Rust toolchain/repository checkout is not available in this execution environment, and the connected GitHub actions toolset does not expose workflow dispatch. Contents commits can trigger repository CI automatically, but the implementation cannot be locally compiled or selectively run from this session. This limitation must not be represented as successful local test execution.

## Acceptance policy

Only mark a requirement complete when the code is integrated, tests exist and are meaningful, and validation evidence is available for the relevant exact commit. This audit is a tracking artifact, not implementation credit.


## Continuation update — 2026-11-19

Verified prior head `a2cac3f3ec09e3b98e6a60311da3063f85a84220`: PR #21 remains Draft; CI #807/#806 and fuzz #354/#353/#352/#351 are successful. Added an isolated trait/impl declaration parser slice in `syntax_v05.rs`: trait signatures, impl generic headers, inherent/trait impl targets, method parameter and return types, body-span capture, duplicate method/parameter rejection and bounded brace-depth. Fuzz target now exercises this parser. New commit `eefd7dc479375a861cd799d0bf9ec25fbb6d2e44` is pending CI after a formatting-only correction. This parser is not compiler-integrated; method bodies are captured, not parsed/compiled. M1.1 remains incomplete and M1.2–M8 remain outstanding.

## Corrected latest-state record — 2026-11-19

The preceding status text is historical and stale. Latest implementation head: `d9270d6c9d6be68439aa1ddee5295325897dcec3`. The trait/impl syntax parser is added but still isolated: it parses signatures, captures method body spans, checks duplicate names and requires `self` as first parameter. CI run #842 is successful on Linux/macOS/Windows (workspace tests, format, Clippy, rustdoc). Fuzz run #371 is still running. Several intermediate CI/fuzz runs failed due formatting mistakes and a missing parser receiver argument; these were corrected, and the current successful CI evidence supersedes those failed attempts. This does not prove complete v0.5 semantics or integration. M1.1 incomplete; M1.2–M8 outstanding.

## Current continuation checkpoint — 2026-11-19

Latest code/fuzz-target commit `bb547f7127499002c634dd30be8938c33af49eb9`. It includes the prior trait/impl parser and a new generic function declaration parser (generic bounds, typed parameters, optional return type, body span, duplicate function/parameter checks, bounded body depth); the `syntax_v05` fuzz target invokes it. CI #858 succeeded on Linux/macOS/Windows (format, workspace tests, Clippy, rustdoc). Fuzz run #378 is in progress. M1.1 remains incomplete; this parser is isolated and function/method bodies remain opaque token spans. M1.2–M8 not implemented.