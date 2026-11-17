# HK 0.4.0 — Implementation Status and Release Gates

**Implementation branch:** feat/hk-v0.4.0-bytecode-vm
**Base contract:** HK 0.3.0
**Normative specification:** HK-BYTECODE-VM-SPEC.md
**Release status:** implementation and verification complete through M6. M7 release approval, merge, tag, and publication remain. This branch is not yet a release.

## Architectural decisions

1. Keep the V0.3 parser and AST as the source front end.
2. Compile AST into versioned in-memory bytecode; do not promise a stable serialized bytecode format in V0.4.
3. Verify every bytecode program before VM execution.
4. Use a bounded stack VM and shared runtime value, binding-cell, environment, and contract primitives.
5. Preserve the V0.3 tree-walking engine as a test-only semantic reference.
6. Link filesystem modules in dependency-first order into one verified program; preserve module-local environments and import/export semantics.
7. Keep public run and run_path signatures unchanged; both public entry points now execute through the VM.
8. Do not introduce unsafe Rust, a JIT, native compilation, persistent bytecode files, or unrelated language features.

## Milestone status

| Milestone | Delivered | Status |
|---|---|---|
| M0 — semantic inventory | V0.3 semantic inventory, compatibility constraints, reference tests, explicit closure-lifetime rules | Complete |
| M1 — shared runtime core | Shared values, mutable arrays, binding cells, environments, native operations, and gradual contracts; reference engine retained for tests | Complete |
| M2 — bytecode and verifier | Versioned program/prototype model, lexical capture descriptors/upvalues, source maps, control-flow/stack/scope verification, malformed-bytecode fuzz target | Complete |
| M3 — AST compiler | Compiler covers all supported source expression/statement families, nested and transitive captures, control flow, arrays, source locations, and gradual-contract metadata | Complete |
| M4 — VM | Verified stack-machine execution, bounded frames/stack, calls/returns, mutable captures, short-circuiting, diagnostics, and V0.3 weak environment lifetime | Complete |
| M5 — modules and typing | Dependency-first module compilation/linking, imports/exports/aliases/live bindings, std:math, source identity, and gradual contracts through VM | Complete |
| M6 — differential/fuzz/robustness | Source and module differential suites; resource and malformed-input regressions; cross-platform CI and full CI fuzz campaign | Complete on final implementation candidate `98fe982`; CI #742 and Fuzz #326 passed |
| M7 — documentation/benchmarks/release | Normative specification, semantic inventory, implementation plan, lexical-capture record, changelog, README, and reproducible benchmark harnesses | Partial: benchmarks and trade-off documentation complete; final consistency review, release approval, tag and GitHub Release remain |

## Compatibility requirements

The V0.3 semantic inventory remains normative. In particular:

- Left-to-right evaluation and && / || short-circuiting.
- Lexical scopes and shadowing; if/while do not create synthetic scopes, for creates one scope.
- Mutable arrays preserve identity and aliasing; equality for arrays/functions is identity-based.
- Constants reject reassignment but do not make an array's contents immutable.
- Function captures share binding cells and preserve the V0.3 weak-environment lifetime behavior. A closure must not artificially keep an expired lexical environment alive.
- Runtime and static gradual-typing contracts remain enforced at declaration, assignment, parameter, and return boundaries.
- Imports are read-only live references; module environments and diagnostics retain their defining source.
- std:math retains abs, ceil, floor, round, sqrt, min, and max.
- Resource limits remain bounded: 64 nested function calls, 65,536 operand-stack entries, 128 lexical/parser nesting levels, 1,024 modules, 1 MiB per module, and 16 MiB aggregate module source.

## Exact-head validation

GitHub Actions completed on implementation candidate `98fe9823e97c9c5813f029f45f42b4cd89cfea03`:

| Check | Result |
|---|---|
| CI — Ubuntu, macOS, Windows | Pass on all three platforms |
| `cargo fmt --all -- --check` | Pass on all three platforms |
| `cargo test --workspace` | 200 passed, 0 failed on each platform |
| `cargo clippy --workspace --all-targets -- -D warnings` | Pass on all three platforms |
| `cargo doc --workspace --no-deps` | Pass on all three platforms |
| Source execution fuzzing | 875,697 runs, no finding |
| Module resolver fuzzing | 88,958 runs, no finding |
| Malformed bytecode verifier fuzzing | 906,550 runs, no finding |

Workflow runs: CI #742 and Fuzz #326. All jobs and steps completed successfully on the implementation candidate above. The candidate includes the documentation consistency corrections and the final compiler-module comment clarification.

The earlier local validation recorded in PR history is supplementary; the exact-head GitHub Actions results above are the release-gate evidence.

## VM benchmark baseline

See HK-0.4-BENCHMARKS.md. The harness measures end-to-end public execution, including parsing, checks, compilation, mandatory verification, and VM execution; the module workload also includes graph loading and linking. It is an absolute baseline, not a performance comparison against V0.3.

## Remaining release gates

1. Re-run cross-platform CI and the full fuzz campaign on the final PR head after documentation corrections.
2. Review the complete final diff, public API, README, specification, changelog, benchmark methodology, and implementation status for consistency; ensure generated build artifacts and fuzz discoveries are not committed.
3. Keep PR #16 in Draft until both exact-head workflows pass and the final review is complete.
4. Only after explicit release approval: mark the PR ready, merge it, create the `v0.4.0` tag, and publish the GitHub Release.

V0.5.0 remains blocked until V0.4.0 has passed these gates and is released.
