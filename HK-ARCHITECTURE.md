# HK Architecture

## Current implementation (0.2.0)

HK is a Rust modular monolith in one Cargo package. The interpreter and gradual-typing implementation are separated into focused modules:

| Module | Responsibility |
|---|---|
| `lexer.rs` | Tokenization, source positions, and type-annotation punctuation |
| `ast.rs` | Expression/statement representation and optional type annotations |
| `parser.rs` | Parsing declarations, control flow, expressions, and type annotations |
| `types.rs` | Source-level type representation and compatibility relation |
| `type_checker.rs` | Lexical type scopes, static expression checks, function signatures, and return-path analysis |
| `runtime.rs` | Tree-walking execution, lexical environments, binding contracts, function contracts, and runtime guards |
| `diagnostics.rs` | Structured diagnostics with source excerpts and caret positions |
| `lib.rs` | Public run pipeline and static-check/runtime orchestration |
| `main.rs` | CLI entry point |

The crate remains intentionally unified. Splitting it into multiple crates is deferred until a real dependency or compilation boundary justifies that cost.

## 0.2.0 execution pipeline

For every source file:

```text
UTF-8 source
  -> Lexer
  -> Parser
  -> AST
  -> Type Checker (when annotations are present)
  -> Tree-walking Runtime
  -> output
```

The checker completes before runtime execution. A lexical, syntax, or static type error prevents program output and runtime side effects.

## Gradual-typing model

The source-level types are `number`, `string`, `bool`, `null`, `array`, and `any`. An omitted annotation remains dynamic; it is not inferred into a permanent binding type.

The checker distinguishes declared binding contracts, static expression facts, and runtime value tags. Runtime contracts are stored on bindings and function boundaries rather than on values or array objects. Arrays remain heterogeneous, and indexed reads are dynamically typed.

Direct named-function calls can use signatures visible at the current source position. Function aliases and unresolved forward calls remain dynamic, while runtime parameter and return contracts stay attached to the actual function value.

## Scope and evaluation invariants

- Explicit blocks create lexical child scopes; `if` and `while` do not add synthetic scopes.
- A `for` loop owns one scope covering initializer, condition, body, and increment.
- Static `for` analysis follows runtime order: initializer, condition, body, increment.
- Short-circuit operators preserve lazy runtime evaluation.
- Dynamic guards do not evaluate expressions more than once.
- Runtime diagnostic classification follows typed-expression provenance; unrelated dynamic regions preserve legacy `E3000` diagnostics.
- Binding writes and function entry/return boundaries enforce contracts before invalid values become observable.

## Runtime safety

Values use a tagged Rust enum. Arrays use shared reference-counted interior mutability. Environments are lexical parent-linked scopes. A constant protects a binding from reassignment, not mutation of an object it references.

The interpreter propagates return control flow explicitly. Invalid HK programs must return diagnostics rather than trigger Rust panics. Parser/runtime depth limits, bounded function recursion, iterative environment traversal, and iterative array-cycle checks remain part of the robustness baseline.

## 0.3.0 target module architecture (design, not yet implemented)

The normative contract is [HK-MODULES-SPEC.md](HK-MODULES-SPEC.md).

The intended pipeline is:

```text
Entry source path
  -> Canonical path resolver / std: registry
  -> Module graph discovery
  -> UTF-8 source loading
  -> Lexer / Parser per module
  -> Export and import linking
  -> Cycle and graph validation
  -> Module-local static checking
  -> Dependency-first module initialization
  -> Shared output collection
```

The implementation must separate these responsibilities rather than embed filesystem access in the parser or evaluator. A module instance owns its canonical identity, source text, AST, export table, and runtime environment. The module graph owns instances for the duration of one `run` invocation. Exported functions must keep their defining module environment alive; imported names must not be implemented by flattening module ASTs into one global scope.

The loader validates the full reachable graph and all import/export links before executing user statements. Cycles are rejected rather than exposing partially initialized values. Filesystem resolution is relative to the importing module, then canonicalized for identity and deduplication. The reserved `std:` namespace is resolved through an explicit standard-library registry, not filesystem fallback.

Imported names are dynamic (`any`) to the 0.3.0 static checker, while runtime binding and function contracts remain attached to the exported values. This is an explicit gradual-typing boundary, not an inference of external signatures.

## Architectural constraints

- Keep the type checker independent of runtime side effects.
- Preserve source locations and source identity through every module.
- Do not add unsafe Rust without a measured requirement and safety review.
- Preserve dynamic behavior for unannotated code.
- Do not flatten modules into a shared global environment.
- Do not execute module bodies during graph discovery or link validation.
- Do not claim bytecode, optimization, native compilation, or performance characteristics that are not implemented and benchmarked.

## Deferred architecture

HIR/MIR, bytecode, optimizer, VM, LLVM/native backend, WASM, package manager, LSP, formatter, debugger, concurrency primitives, and C/C++ FFI are future work. They are not prerequisites for 0.2.0 or 0.3.0.
