# HK 0.5.0 — Design and Implementation Plan

**Current phase:** D0 — design and semantic decisions.
**Implementation status:** not started.
**Base:** merged HK 0.4.0 commit `c47a06cec1120edd33114cce1857c40f10b7f333`.
**Normative candidate:** [HK-0.5-SPEC.md](HK-0.5-SPEC.md).

## Release strategy

HK 0.5.0 is a feature release, not a feature grab-bag. The release order is: settle semantics, validate grammar/type rules, implement the runtime representation, integrate bytecode/verifier/VM, then expose module support. No feature is considered delivered merely because its syntax parses.

## Milestones and exit gates

| Milestone | Scope | Exit evidence |
|---|---|---|
| D0 — design closure | Syntax, receiver semantics, enum inspection, generic strategy, trait coherence, dynamic boundaries | Reviewed normative spec; no unresolved semantic blockers |
| M1 — front-end model | AST, parser, nominal type table, generic type references, diagnostics | Parser golden tests; duplicate/invalid declaration tests; deterministic declaration collection |
| M2 — static semantics | Nominal checking, field/method resolution, enum payload checking, trait conformance, generic constraints/inference | Positive/negative checker matrix; no regression in gradual typing |
| M3 — runtime values | Struct/enum tagged representations, field access/update, enum construction/inspection, identity/aliasing | Runtime semantics tests; aliasing and invalid dynamic boundary tests |
| M4 — bytecode and verifier | Constants/descriptors/opcodes, method call targets, aggregate layouts, verifier checks | Adversarial malformed-bytecode tests; verifier rejects before VM entry |
| M5 — VM execution | Verified execution of aggregate and method operations, bounded generic metadata handling | VM tests and tree-walker differential parity where supported |
| M6 — modules | Export/import nominal types and traits; cross-module coherence and type resolution | Multi-module integration tests; deterministic failure before module side effects |
| M7 — hardening | Fuzzing, resource limits, diagnostics, cross-platform CI, benchmarks | No fuzz findings; CI green on all supported OSes; documented costs |
| M8 — release readiness | API/docs/changelog/README review, migration notes, exact-head validation | Final review checklist, approved PR, merge, tag, GitHub Release |

## Required design decisions (D0)

1. **Receiver model:** choose value/reference semantics compatible with HK's shared mutable binding cells; document field mutation and aliasing.
2. **Enum operations:** decide whether 0.5 includes exhaustive `match`, minimal tag/payload inspection, or construction only. Avoid half-specified destructuring.
3. **Generic implementation:** compare monomorphization with erased descriptors. Evaluate bytecode size, VM dispatch, recursion, verifier proof obligations, error locations, and module linking.
4. **Trait dispatch:** use compile-time resolution unless evidence requires runtime dispatch. Define coherence/orphan rules across modules.
5. **Inference:** specify constraints, unification, ambiguity, expected-type propagation, and behavior through `any`.
6. **Resource limits:** cap type recursion, generic instantiations, and aggregate layout expansion with deterministic diagnostics.
7. **Compatibility:** all unannotated 0.4 programs preserve behavior; new typed boundaries must not silently reinterpret existing primitive types.

No implementation milestone starts while any of these decisions can change observable semantics.

## Test strategy

- Parser: valid grammar, malformed declarations, precedence/ambiguity, source spans.
- Type checker: nominal identity, field access, method resolution, generic inference, bounds, trait conformance, cross-module names, `any` boundaries.
- Runtime: value identity, aliasing, field mutation, enum payload integrity, recursion/resource exhaustion.
- Bytecode verifier: invalid descriptor indices, mismatched layouts, invalid variant tags/arity, invalid method targets, inconsistent stack effects, forged generic metadata.
- Modules: import/export visibility, duplicate implementation conflicts, deterministic errors before side effects.
- Differential: compare old language subset against retained reference semantics; add aggregate semantics to reference engine before claiming differential coverage.
- Fuzzing: parser/type resolver, generic-instantiation stress, module type graph, malformed bytecode.
- Performance: release-mode baselines separating parse/check/compile/verify/execute; report code-size and memory effects where measurable.

## Release blockers

- Any unresolved D0 decision.
- Any verifier path that trusts bytecode type/aggregate metadata without validating it.
- Any regression in existing 0.4 language behavior without an explicit compatibility decision.
- Nondeterministic trait resolution or cross-module linking.
- Unbounded recursive generic expansion or aggregate construction.
- CI, fuzz, or documentation mismatch.

## Work discipline

Each milestone is delivered in a focused PR with exact acceptance tests. Keep the public surface minimal. Do not introduce syntax sugar, unrelated refactors, or speculative optimization into feature PRs. Update this plan as evidence changes; never mark a milestone complete without linked commits and passing checks.
