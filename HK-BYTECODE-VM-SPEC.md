# HK 0.4.0 — Versioned Bytecode and Virtual Machine Specification

**Status:** Normative HK 0.4.0 specification. Implemented on `feat/hk-v0.4.0-bytecode-vm`; final release gates are tracked in `HK-0.4-IMPLEMENTATION-PLAN.md`.
**Baseline:** HK 0.3.0, gradual typing, filesystem modules, `std:math`.
**Primary objective:** Introduce a verified stack-based bytecode execution engine without changing observable HK language semantics.

## 1. Scope and non-goals

HK 0.4.0 introduces:
1. An AST-to-bytecode compiler.
2. A versioned, in-memory bytecode program representation.
3. A structural and control-flow bytecode verifier.
4. A deterministic stack-based virtual machine.
5. VM integration with functions, lexical scopes, runtime contracts, arrays, native functions, and HK 0.3.0 modules.
6. Differential conformance testing against the existing tree-walking interpreter.

The bytecode format is an internal compiler/runtime contract in 0.4.0. It is versioned in memory, but **not a stable serialized file format**. There is no `.hkb` file extension, bytecode loader, cross-process persistence guarantee, or compatibility promise for bytecode artifacts in this release.

Explicit non-goals:
- JIT, native code generation, LLVM, WASM, FFI, or hardware-specific instructions.
- Optimizing compiler passes beyond safe constant representation and instruction lowering.
- New source-language syntax or semantics.
- Closures as a new language feature, anonymous functions, lambdas, generics, classes, traits, exceptions, coroutines, or concurrency.
- Package manager, dynamic imports, cyclic modules, or new module semantics.
- Persistent bytecode serialization or third-party bytecode execution.
- Performance claims without reproducible benchmarks.

## 2. Compatibility contract

The VM must preserve the observable semantics of HK 0.3.0 for every accepted program:
- lexical scope, shadowing, declaration order, mutability, and duplicate-declaration diagnostics;
- function declaration, recursion, argument evaluation order, return behavior, and call diagnostics;
- left-to-right expression and argument evaluation;
- short-circuit `&&` and `||` without evaluating the unused operand;
- arithmetic, finite-number enforcement, comparison, equality, string concatenation, array identity, array mutation, indexing, and property access;
- gradual type checking, type annotations, static diagnostics, runtime binding/parameter/return contracts, and typed-provenance diagnostics;
- import/export linking, isolated module environments, live read-only aliases, defining-module function environments, dependency-first initialization, output order, and at-most-once initialization;
- `std:math` behavior and diagnostic codes;
- source file, line, and column information in diagnostics.

The parser and AST remain the source-language front end. The VM must not re-parse source or infer semantics from source text.

The existing tree-walking engine remains available internally as the reference implementation throughout 0.4.x. Public `run` and `run_path` switch to VM execution only after the complete release gate passes. An internal/test-only reference entry point is retained for differential tests; it is not required to become a permanent public API.

## 3. Execution pipeline

Single-source and filesystem-backed programs converge on a common compiled-program representation:

```text
HK source
  -> Lexer
  -> Parser
  -> AST
  -> Gradual Type Checker (when annotations are present)
  -> Bytecode Compiler
  -> Bytecode Program
  -> Bytecode Verifier
  -> Virtual Machine
  -> output / structured diagnostics
```

For filesystem programs, the existing module loader still discovers and validates the complete graph before any user module body executes. Each module is compiled into a module unit in one linked `BytecodeProgram`. Import aliases resolve to export binding cells; module initialization order remains the 0.3.0 dependency-first order.

The VM must never execute unverified bytecode. Verification happens after compilation/linking and before the first instruction of user code.

## 4. Versioned bytecode model

The in-memory program contains:
- `BytecodeHeader`: bytecode format major/minor version, required feature bits, and compiler compatibility metadata;
- `ConstantPool`: finite numbers, strings, and other immutable literal payloads;
- `FunctionProto`: function identity/name, parameter-to-slot metadata, local-slot descriptors (internal/source name, mutability, optional contract), optional return contract, typed-function provenance, maximum operand stack, instruction vector, and module identity;
- `InstructionStream`: compact opcode plus typed operands;
- `ModuleTable`: module identity, initializer function, import links, export binding slots, and source identity;
- `SourceMap`: instruction-offset to source identity, line, and column mappings;
- `Global/Module Binding Layout`: binding slots, mutability, optional runtime contract, and import alias metadata.

Versioning rules:
- A bytecode program declares one exact format version.
- Unknown major versions are rejected.
- Unsupported required feature bits are rejected.
- Minor-version compatibility is allowed only when all required feature bits are understood and verifier rules remain compatible.
- The VM rejects malformed indexes, invalid opcodes, invalid control-flow targets, inconsistent stack states, and invalid function/module references before execution.
- No unsafe memory access or unchecked bytecode decoding is permitted.

The bytecode format is not a public persistence ABI in 0.4.0. Version metadata exists to make compiler/VM mismatch explicit and to prepare for a future separately specified serialized format.

## 5. Operand stack and frames

The VM is a deterministic stack machine:
- Each call frame owns an instruction pointer, function prototype, local-slot storage, lexical-scope state, and base operand-stack offset.
- The operand stack stores HK runtime values, not untyped raw memory.
- Function calls create frames; returns restore the caller frame and push the return value.
- Each function has a statically verified maximum operand-stack depth.
- The VM enforces a runtime maximum call depth and bounded operand-stack growth.
- Frame and stack limits produce structured HK runtime diagnostics, never Rust panics or process aborts.
- Function arguments are evaluated exactly once, left-to-right, before call dispatch and contract validation.
- Return contracts are checked before the caller observes the returned value.
- The VM preserves function source identity and defining module environment.

Lexical bindings are represented by compiler-assigned slots and runtime binding cells. Captured lexical environments required by existing named-function behavior must remain alive for the same lifetime as in 0.3.0. This is an implementation requirement, not a new promise of anonymous closures or lambda syntax.

## 6. Initial opcode families

The initial instruction set is deliberately small and semantic, not tied to a CPU architecture.

### Constants and stack
- `Const`, `Null`, `True`, `False`
- `Pop`, `Dup`

### Bindings and scopes
- `DefineLocal`, `GetLocal`, `SetLocal`
- `GetModule`, `SetModule`
- `EnterScope`, `ExitScope`
- `GetImport` or a verified binding-slot equivalent

### Arithmetic and logic
- `Negate`, `Not`
- `Add`, `Subtract`, `Multiply`, `Divide`, `Modulo`
- `Equal`, `NotEqual`, `Greater`, `GreaterEqual`, `Less`, `LessEqual`

### Control flow
- `Jump`, `JumpIfFalse`
- `Call`, `Return`

### Arrays and properties
- `BuildArray`, `GetIndex`, `SetIndex`, `GetProperty`

### Runtime and native boundaries
- `CheckBindingContract`, `CheckParameterContract`, `CheckReturnContract` (or equivalent verified metadata-driven checks)
- `CallNative` or native callable values
- `Print` may remain a native function call; there must not be a second inconsistent print implementation.

Exact opcode names and operand encodings may change during implementation, but the semantic obligations and verifier invariants in this document are normative. Do not add speculative opcodes for language features not in 0.3.0.

## 7. Compiler requirements

The compiler lowers every valid AST construct currently supported by HK 0.3.0:
- untyped and typed variable declarations;
- named function declarations and recursion;
- expression statements and returns;
- blocks and lexical scopes;
- `if/else`, `while`, and `for`;
- all unary/binary operators and short-circuit operators;
- calls, arrays, indexing, array assignment, and property access;
- top-level module imports/exports and module initializer bodies.

The compiler must:
- preserve source evaluation order exactly;
- emit source locations for every potentially failing operation;
- lower short-circuit expressions with control flow rather than eager binary instructions;
- allocate deterministic local slots and preserve shadowing;
- retain runtime contract metadata at binding and function boundaries;
- emit no executable code for import declarations themselves; module linking is represented in the module table;
- reject unsupported or internally inconsistent AST states with compiler diagnostics, not panics;
- calculate or allow verification of stack depth and control-flow consistency.

The compiler is not allowed to silently drop AST statements or downgrade typed code to dynamic behavior.

## 8. Bytecode verifier

The verifier is a mandatory security and correctness boundary, even though 0.4.0 executes only compiler-produced in-memory bytecode.

It must validate:
1. Header version and required features.
2. Opcode validity and operand encoding.
3. Constant, function, module, binding, and primary/operand source-map indexes.
4. Jump targets are within the current function and land on instruction boundaries.
5. Control-flow stack-height consistency at every merge point.
6. No stack underflow on any reachable path.
7. Function return paths leave exactly the required result and do not leak frame state.
8. Local-slot accesses are within the declared frame layout.
9. Call operand/argument counts are valid and bounded.
10. Module initializer references and import/export binding links are valid.
11. Maximum stack and code-size limits are respected.
12. No control-flow edge can jump between function bodies or module units.

Verification uses a worklist over the control-flow graph and propagates abstract operand-stack heights plus definite-initialization state for local slots. Jump targets are instruction indexes in the in-memory instruction stream, never arbitrary byte offsets. If future opcodes require richer abstract state, the verifier must extend the abstract domain explicitly.

The verifier must not trust compiler-supplied `max_stack`; it recomputes the maximum and rejects mismatches. It validates every primary and operand source location. Mutability, imported-alias write failures, and duplicate declaration behavior remain runtime semantics; rejecting such writes statically would change behavior for unreachable code.

## 9. Runtime values and environments

The tree-walking engine and VM must share one authoritative runtime value model or a carefully audited compatibility layer. Do not maintain divergent implementations of numeric finiteness, equality, array identity, array-cycle handling, native functions, or type-contract checks.

Recommended refactor:
- extract runtime values, arrays, binding cells, contracts, and native callable metadata into focused internal modules;
- keep tree-walking evaluation and VM dispatch as separate engines;
- centralize operations such as arithmetic, equality, index validation, finite-number checks, contract enforcement, and value formatting;
- preserve reference-counted array identity and mutation semantics;
- preserve imported aliases as live, read-only references to export binding cells.

No unsafe Rust is allowed in the initial VM implementation. Any future unsafe optimization requires a separate measured proposal and a safety review.

## 10. Module execution

The existing 0.3.0 module loader remains the authoritative source of module resolution and graph validation. The VM integration must:
- compile each validated module to a module initializer and function prototypes;
- preserve canonical module identity and deterministic module IDs;
- link imports to export binding cells before execution;
- preserve dependency-first, source-import-order initialization;
- initialize each module once per `run_path` call;
- reject cycles and missing exports before executing any module body;
- preserve output aggregation and source-aware diagnostics;
- keep module cache lifetime scoped to one execution.

Single-source `run(source_name, source)` remains supported and does not gain filesystem import behavior.

## 11. Diagnostics and resource limits

Existing diagnostic codes and source locations remain stable where the same semantic error is reported. New VM-specific failures use a reserved, documented diagnostic range and include:
- bytecode verification failure (internal/compiler defect);
- call-stack limit exceeded;
- operand-stack/resource limit exceeded;
- invalid VM state that can be reported safely.

An invalid bytecode verification result is an internal compiler/VM error and must be distinguishable from ordinary user runtime errors. It must never be reported as successful execution.

Resource limits must be explicit, deterministic, documented, and tested. Initial ceilings must be selected from stress-test evidence before implementation is marked complete. Limits must not depend on host pointer width or trigger stack overflow in Rust itself.

## 12. Differential conformance and performance

The reference tree-walking engine and VM run the same corpus. For each program, tests compare:
- successful output, in exact order;
- error code and semantic classification;
- source identity, line, and column;
- side-effect boundaries (e.g. short-circuiting, failed contracts, module initialization);
- arrays, module state, and function behavior observable through output.

Where diagnostics necessarily differ because a VM-specific internal failure is introduced, the test must assert that difference explicitly and document why.

Benchmarks are required for representative workloads: arithmetic loops, recursive calls, array construction/indexing/mutation, function calls, and multi-module startup. Report compiler time separately from VM execution time. Do not claim a speedup unless repeated measurements on a documented machine show it. Performance is an evaluation criterion, not a semantic acceptance shortcut.

## 13. Public API and rollout

- Preserve the existing public `hk::run` and `hk::run_path` signatures.
- Introduce internal `compile`, `verify`, and `execute_bytecode` boundaries.
- Do not expose mutable raw bytecode construction as a stable public API in 0.4.0.
- Keep a test-only/reference path to the tree-walking engine.
- The V0.4 branch switches `run` and `run_path` to the VM after implementation and compatibility gates; the exact-head CI and fuzz campaign have now passed.
- If a new semantic mismatch is discovered during final review, block release, preserve the reference engine, and fix the mismatch before declaring V0.4.0 complete.

## 14. Acceptance criteria

### Compiler
- [ ] Every AST construct accepted by 0.3.0 is compiled or rejected with a documented compiler error; none is silently omitted.
- [ ] Source positions are preserved for all executable instructions and diagnostics.
- [ ] Lexical slots preserve scope, shadowing, mutability, and declaration errors.
- [ ] Short-circuit evaluation is lazy and exactly-once.
- [ ] Function calls preserve left-to-right evaluation and contracts.

### Verifier
- [ ] Invalid opcode, index, jump, stack underflow, inconsistent stack merge, invalid/uninitialized local access where statically provable, invalid function reference, and invalid module link are rejected.
- [ ] Malformed bytecode fuzzing cannot panic, hang, or access invalid memory.
- [ ] Verification runs before any user instruction or module initializer.

### VM
- [ ] All 0.3.0 language constructs execute correctly.
- [ ] Runtime value, array identity, mutation, equality, and numeric behavior match the reference engine.
- [ ] Runtime contracts and diagnostic provenance match.
- [ ] Call/operand resource limits fail safely and deterministically.

### Modules
- [ ] Filesystem modules, standard-library imports, live aliases, module isolation, dependency order, and at-most-once initialization match 0.3.0.
- [ ] Cycles and link errors are rejected before side effects.
- [ ] Imported functions preserve their defining module environment and source locations.

### Quality gates
- [ ] Existing 0.1.x–0.3.0 tests remain intact and pass.
- [ ] Differential corpus has zero unexplained semantic mismatches.
- [ ] Compiler and verifier have dedicated unit/integration tests.
- [ ] Fuzz targets cover compiler input, verifier bytecode, VM execution, and module linking.
- [ ] Linux, Windows, and macOS CI pass formatting, tests, Clippy `-D warnings`, and rustdoc.
- [ ] Benchmarks are reproducible and make no unsupported performance claim.
- [ ] README, architecture, changelog, roadmap, and implementation plan agree on shipped status.
- [ ] No blocking review threads remain before merge.

## 15. Release policy

No Alpha-number treadmill. Work proceeds through coherent implementation milestones on one V0.4 development branch. Merge only when all acceptance criteria and exact-head CI/fuzz gates pass. Publish `v0.4.0` only after merge and explicit release verification.
