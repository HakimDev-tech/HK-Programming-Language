# HK 0.4.0 — Implementation Plan and Release Gates

**Status:** Design baseline only. No V0.4 runtime or compiler implementation has been merged.
**Target:** HK 0.4.0 — Versioned Bytecode and Virtual Machine
**Base:** HK 0.3.0 on `main`
**Normative contract:** [HK-BYTECODE-VM-SPEC.md](HK-BYTECODE-VM-SPEC.md)

## 1. Architectural decisions

1. AST remains the source front end; no parser rewrite.
2. AST-to-bytecode compilation is a distinct stage.
3. Bytecode is versioned in memory, not serialized as a stable user artifact in 0.4.0.
4. Bytecode verification is mandatory before execution.
5. The VM is a stack machine with bounded frames and operand stack.
6. Runtime values, arrays, binding cells, and contract operations are shared between VM and tree-walking reference engine.
7. Existing module loader semantics remain authoritative; module units are linked into one verified bytecode program.
8. Tree-walking execution remains as an internal reference until differential validation is complete.
9. Public `run` and `run_path` signatures remain unchanged; they switch to VM only after all release gates pass.
10. No unsafe Rust, JIT, native backend, persistent bytecode format, or speculative language features.

## 2. Implementation milestones

### M0 — Baseline and semantic inventory
- Record exact HK 0.3.0 baseline and current CI/fuzz status.
- Enumerate every AST variant, runtime operation, diagnostic code, and module behavior.
- Convert the 0.3.0 regression suite into a VM differential corpus.
- Define deterministic VM resource ceilings from stress-test evidence.

**Exit:** No AST/runtime semantic case is unaccounted for.

### M1 — Runtime semantic core extraction
- Extract shared value representation, array identity/mutation, binding cells, aliases, contracts, native callables, arithmetic, equality, formatting, and diagnostics helpers.
- Keep the tree-walking engine behavior unchanged.
- Add unit tests for extracted operations and prove existing test suite remains unchanged.

**Exit:** Reference interpreter and extracted core pass all pre-existing tests.

### M2 — Bytecode representation and verifier
- Define header/version/features, constants, function prototypes, module table, instruction stream, source maps, and binding layout.
- Implement decoder-free in-memory verifier and CFG stack-height analysis.
- Add negative tests for every verifier rejection rule and fuzz target for arbitrary bytecode structures.

**Exit:** Malformed bytecode is rejected before VM execution; verifier fuzzing has no panic/hang.

### M3 — Compiler foundations
- Implement deterministic constant pools, function prototypes, source maps, lexical slot allocation, scope handling, and expression lowering.
- Add compiler golden tests for representative AST programs.
- Lower short-circuit operators with jumps.

**Exit:** All compiler golden tests pass; verifier accepts every compiler output.

### M4 — VM core
- Implement operand stack, frames, local/module binding access, jumps, arithmetic, arrays, calls, returns, and native call dispatch.
- Enforce contracts and resource limits.
- Preserve source-aware errors.

**Exit:** VM unit tests pass and simple compiled programs match the reference engine.

### M5 — Language and module coverage
- Add loops, nested scopes, named functions, recursion, arrays/properties, gradual typing, native math, imports/exports, live aliases, and module initialization.
- Verify whole linked module program before any initializer executes.

**Exit:** Full feature suite matches reference behavior.

### M6 — Differential, fuzz, and robustness
- Run all existing examples and tests through both engines.
- Add generated programs where bounded and semantically valid.
- Fuzz compiler inputs, bytecode verifier, VM instruction execution, and module linking.
- Stress recursion, operand-stack growth, nested expressions, arrays, and large module graphs.

**Exit:** Zero unexplained differential mismatches; no crash/hang in fuzz/stress tests.

### M7 — Benchmarks and rollout
- Benchmark interpreter vs compile+VM separately for arithmetic, calls, arrays, and module startup.
- Audit all public API behavior and diagnostic changes.
- Switch public APIs to VM only if all gates pass.
- Update documentation and release notes.

**Exit:** Exact candidate CI and fuzz pass on Linux/Windows/macOS; benchmark results and known limitations documented.

## 3. Repository structure proposal

Add focused internal modules, keeping the single Cargo crate:
- `bytecode.rs`: program header, opcodes, constants, function prototypes, module table.
- `compiler.rs`: AST lowering, lexical resolution, function and module compilation.
- `verifier.rs`: opcode validation, operand validation, CFG and stack-height analysis.
- `vm.rs`: dispatch loop, operand stack, call frames, resource accounting.
- `value.rs` / `environment.rs` (or similarly scoped modules): shared values, arrays, binding cells, aliases, contracts.
- `runtime_ops.rs`: shared numeric, comparison, index, formatting, and native-call semantics.
- Keep `runtime.rs` as the reference tree-walking engine during migration.
- Keep `module_loader.rs` responsible for filesystem discovery and graph validation; add a compilation/linking boundary rather than duplicating path resolution.

Names are provisional; implementation may adjust file boundaries if responsibilities remain explicit and testable.

## 4. Test matrix

| Area | Required evidence |
|---|---|
| AST coverage | One or more compiler + VM tests per AST variant |
| Stack verifier | Underflow, merge mismatch, bad jump, bad index, bad function/module refs |
| Scope | Shadowing, nested blocks, for-loop scope, duplicate declarations |
| Expressions | Every operator, evaluation order, lazy boolean operators |
| Functions | Recursion, arity, contracts, return, source mapping |
| Arrays | Construction, identity equality, nested values, index read/write, bounds |
| Gradual typing | Typed/untyped boundaries, binding/parameter/return guards, diagnostics |
| Modules | Imports, exports, aliases, isolation, live bindings, cycles, output order |
| Stdlib | All std:math operations and error cases |
| Safety | Stack/frame limits, verifier fuzzing, VM/compiler fuzzing |
| Platforms | Linux, Windows, macOS CI |
| Performance | Reproducible benchmark suite with compile time separated |

## 5. Release gates

- [ ] All acceptance criteria in the normative spec pass.
- [ ] Existing 0.3.0 tests are retained; no test is weakened or removed to obtain a pass.
- [ ] Reference-vs-VM differential suite has zero unexplained mismatches.
- [ ] Verifier rejects malformed bytecode before side effects.
- [ ] Compiler and verifier fuzz campaigns pass on exact candidate commit.
- [ ] VM and module execution fuzz/stress campaigns pass on exact candidate commit.
- [ ] Linux, Windows, macOS: fmt, full tests, Clippy `-D warnings`, rustdoc.
- [ ] Benchmarks and resource limits documented.
- [ ] Public APIs and diagnostics audited.
- [ ] Docs and crate version agree on 0.4.0.
- [ ] PR review has no unresolved blocking comments.
- [ ] Merge and publish tag/release only after all checks pass.

## 6. Explicitly deferred

Persistent bytecode serialization, external bytecode loading, stable bytecode ABI, JIT, native compilation, WASM, FFI, closures as a new language feature, anonymous functions, generics, classes, traits, exceptions, concurrency, package management, LSP, formatter, and debugger.
