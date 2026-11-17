# HK 0.4.0 — Implementation Status and Release Gates

**Implementation branch:** feat/hk-v0.4.0-bytecode-vm
**Base contract:** HK 0.3.0
**Normative specification:** HK-BYTECODE-VM-SPEC.md
**Release status:** implementation complete through M5; M6/M7 final release gates remain open. This branch is not yet a release.

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
| M6 — differential/fuzz/robustness | Source and module differential suites; targeted local fuzz campaigns; resource and malformed-input regressions | Partial: longer CI fuzz campaign and final CI run on the release candidate remain |
| M7 — documentation/benchmarks/release | Documentation and reproducible VM benchmark harness/baseline added | Partial: V0.3 comparison, final PR CI review, release approval, tag and GitHub Release remain |

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

## Validation performed on the current local implementation

| Check | Result |
|---|---|
| cargo fmt --all -- --check | Pass |
| cargo test --workspace --all-targets | 198 passed, 0 failed |
| cargo test --workspace --all-targets --release | 198 passed, 0 failed |
| cargo clippy --workspace --all-targets -- -D warnings | Pass |
| cargo check --manifest-path fuzz/Cargo.toml --all-targets | Pass |
| RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps | Pass |
| git diff --check | Pass |
| Fuzz run target (source parser + public VM) | 74,570 runs, 12-second campaign, no finding |
| Fuzz modules target | 24,239 runs, 12-second campaign, no finding |
| Fuzz bytecode_verifier target | 171,733 runs, 12-second campaign, no finding |

The three fuzz runs above were short local smoke campaigns using the installed nightly toolchain. They do not replace the longer scheduled/CI campaigns. A first combined fuzz invocation was interrupted by the sandbox execution limit; each target was then run separately to completion with exit code 0.

## VM benchmark baseline

See HK-0.4-BENCHMARKS.md. The harness measures end-to-end public execution, including parsing, checks, compilation, mandatory verification, and VM execution; the module workload also includes graph loading and linking. It is an absolute baseline, not a performance comparison against V0.3.

## Remaining release gates

1. Run the full CI workflow on the exact final PR head, including Linux/Windows/macOS.
2. Complete the longer CI fuzz durations (source: 45 s; modules: 30 s; verifier: 30 s) on the exact final PR head.
3. Establish a reproducible V0.3 tree-walker versus V0.4 VM benchmark comparison on the same host and toolchain; do not make speed claims without it.
4. Review the full diff and public API/docs for unintended changes; ensure generated build artifacts and fuzz discoveries are not accidentally committed.
5. Update PR #16 from Draft only after the above gates pass. Keep it unmerged until the explicit release decision.
6. Create the V0.4.0 tag and GitHub Release only after merge and final approval.

V0.5.0 remains blocked until V0.4.0 has passed these gates and is released.
