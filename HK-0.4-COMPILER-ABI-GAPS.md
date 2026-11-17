# HK 0.4.0 — Lexical capture implementation and ABI invariants

**Status:** Implemented across M2 (bytecode/verifier), M3 (compiler), and M4 (VM). This document records the compatibility contract and the implementation now present on `feat/hk-v0.4.0-bytecode-vm`; it is no longer a list of unimplemented compiler/VM obligations.

## 1. V0.3 lexical-environment contract

The V0.3 interpreter stores a function's defining lexical environment as a `Weak<RefCell<Env>>` in `Fun::env`. Nested named functions can resolve and mutate ancestor bindings while the defining environment is alive. The module loader separately keeps defining module environments alive for imported functions during one `run_path` execution.

HK 0.4 preserves these ownership and lifetime boundaries. It does not turn the V0.3 weak lexical-environment relationship into an indefinitely strong escaping-closure guarantee. If the defining environment expires, execution retains the reference runtime's controlled `Function environment is no longer available` failure.

## 2. Bytecode ABI

`FunctionProto` contains:
- `enclosing_function: Option<u32>` — lexical parent prototype, absent for roots;
- `upvalues: Vec<UpvalueDescriptor>` — ordered capture table;
- each descriptor records `UpvalueSource::EnclosingLocal(slot)` or `UpvalueSource::EnclosingUpvalue(slot)`, mutability, and an optional type contract.

The opcode set contains `GetUpvalue(u16)` and `SetUpvalue(u16)`. Captures refer to shared binding cells rather than copied values. The compiler ABI version is 2; programs using upvalues declare `FEATURE_LEXICAL_UPVALUES`. The verifier rejects unsupported ABI versions and required feature bits.

## 3. Verifier guarantees

The verifier enforces:
- valid earlier lexical parent prototypes for capture-bearing functions;
- valid local and forwarded capture sources;
- exact capture mutability and type-contract metadata;
- unique capture descriptors;
- nested-prototype instantiation only by the declared lexical parent;
- definite initialization before a parent local is captured;
- valid upvalue indexes and stack effects;
- control-flow, operand-stack, and lexical-scope invariants for capture instructions.

Writes to immutable bindings remain runtime-enforced, preserving the reference interpreter's behavior for unreachable writes.

## 4. Compiler and VM obligations — implemented

The compiler and VM now implement and test:
- shared mutable binding cells, capture identity, and contract preservation;
- nested and transitive capture forwarding;
- self-recursive and mutually recursive named functions;
- lexical shadowing, scope exit, and weak environment expiration;
- captured const/import write failures at the runtime boundary;
- typed variable, parameter, and return contracts through captures;
- native `print` and all `std:math` functions, with lexical shadowing and arity/type/domain diagnostics;
- lowering for supported AST expression and statement families, with explicit diagnostics at the source-only import boundary;
- mandatory verification before public VM execution.

The compiler emits source-aware instructions and gradual-typing provenance. Public `hk::run` and `hk::run_path` execute through the verified VM; the tree-walking implementation remains available as the differential reference.

## 5. Evidence and remaining release controls

The compiler-focused tests include AST-family coverage, typed metadata, loop scopes, module-boundary diagnostics, and top-level return diagnostics. VM and module integration tests compare output or structured failures against the reference engine, including capture lifetime, mutable captures, cross-module functions, aliases, live bindings, source identity, and gradual-contract boundaries.

On commit `392fe58c04392a32ef20df5549f5fe172b1ad490`, GitHub Actions completed successfully:
- CI on Ubuntu, macOS, and Windows;
- source execution fuzzing: 875,697 runs;
- module resolver fuzzing: 88,958 runs;
- malformed bytecode verifier fuzzing: 906,550 runs.

These results close the implementation obligations described here. They do not by themselves authorize release: the final PR diff/documentation review, draft-to-ready transition, merge, tag, and GitHub Release remain governed by `HK-0.4-IMPLEMENTATION-PLAN.md`.
