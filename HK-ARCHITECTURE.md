# HK Architecture

## Current baseline: HK 0.4.0

HK is a Rust modular monolith in one Cargo package. The 0.4 execution pipeline is:

```text
UTF-8 source -> Lexer -> Parser -> AST -> Gradual Type Checker
  -> AST-to-bytecode Compiler -> Mandatory Bytecode Verifier
  -> Versioned In-Memory Bytecode -> Stack VM -> output
```

The public `hk::run` and `hk::run_path` entry points execute through verified bytecode. The tree-walking runtime is retained as a semantic reference for differential testing. The bytecode is an in-memory, versioned representation; it is not a stable serialized artifact.

## Core subsystems

| Subsystem | Responsibility |
|---|---|
| Lexer / parser / AST | Source syntax, spans, and structured declarations |
| Types / type checker | Gradual type contracts and static checks |
| Compiler | Lower AST into versioned bytecode and source maps |
| Bytecode verifier | Validate control flow, stack effects, scopes, captures, and references before execution |
| VM | Bounded stack-based execution, calls, closures, and runtime operations |
| Runtime core | Tagged values, binding cells, environments, contracts, arrays, native functions |
| Module loader/linker | Deterministic module discovery, validation, linking, and dependency-first execution |
| Diagnostics | Stable error categories with source-aware excerpts |
| Public API / CLI | Source and filesystem execution surfaces |

The crate remains intentionally unified. Splitting into multiple crates is deferred until a real dependency or compilation boundary justifies that cost.

## Gradual typing

The source-level primitive types are `number`, `string`, `bool`, `null`, `array`, and `any`. Omitted annotations remain dynamic. The checker distinguishes declared binding contracts, static expression facts, and runtime value tags. Runtime contracts are enforced at typed binding and function boundaries.

## Modules

Each filesystem module has an independent root lexical environment. Imports are named, explicit, and restricted to the module header; exports are explicit. Relative paths resolve from the importer, canonical module identities are cached once per run, cycles are rejected before execution, and dependencies initialize deterministically. Imported aliases are read-only live references. The `std:` namespace uses an explicit registry; `std:math` is supported.

## Runtime and safety invariants

- Values use tagged Rust representations; arrays retain shared identity and interior mutability.
- Lexical scopes and closure captures preserve the established 0.3 semantics.
- Parser, runtime, VM stack/call depth, and module graph work are bounded.
- The bytecode verifier is mandatory for every VM execution path.
- No unsafe Rust is introduced without a measured requirement and safety review.
- Source identity and spans survive compilation and module linking.
- Do not claim serialized bytecode, JIT, native compilation, or performance improvements unless implemented and measured.

## 0.5 design direction — not implemented

The 0.5 proposal is documented in [HK-0.5-SPEC.md](HK-0.5-SPEC.md). It evaluates nominal structs/enums, explicit traits, and bounded generics. The proposed additions must extend the front end, type checker, runtime value model, bytecode descriptors, verifier, VM, and module linker as one coherent feature set.

Key architectural gates:
- settle struct/enum identity, mutation, receiver, and payload semantics;
- select a generic implementation strategy only after comparing verifier complexity, code size, runtime cost, and module behavior;
- resolve trait methods deterministically at compile time; trait objects/dynamic dispatch are not assumed;
- validate all type/aggregate descriptors before VM execution;
- preserve 0.4 behavior for existing unannotated programs.

No 0.5 implementation claim is made by this architecture document.
