# HK Architecture

## Current implementation baseline: 0.1.x

HK is a Rust modular monolith in one Cargo package. The implementation is split into lexer, AST, parser, runtime, and diagnostics modules; the CLI remains in `main.rs`. This avoids premature crate boundaries while preserving separable responsibilities.

TypeScript is reserved for future developer tooling (playground, editor integration, website). C++ is reserved for optional native interoperability or measured low-level components. Neither is a dependency of the current interpreter.

## 0.1.x execution pipeline

```text
UTF-8 source -> Lexer -> Tokens with locations -> Parser -> AST
             -> Tree-walking interpreter -> Runtime values/scopes -> output
```

## Runtime baseline

Values use a tagged Rust enum. Arrays use shared reference-counted interior mutability. Environments are lexical parent-linked scopes. A constant protects a binding from reassignment, not the mutability of an object it references.

The interpreter propagates return control flow explicitly. Invalid HK programs must return diagnostics rather than trigger Rust panics. Existing parser/runtime limits and iterative graph/environment traversals are part of the robustness baseline.

## 0.2.0 target architecture — design only

The 0.2.0 design adds a static type-checking phase and runtime contract guards while retaining the Tree-Walking interpreter.

```text
UTF-8 source
  -> Lexer
  -> Parser
  -> AST
  -> Type Checker
  -> Runtime with typed binding/function contracts
  -> output
```

No program executes and no output is produced if parsing or static type checking fails.

### Planned module responsibilities

| Module | Responsibility |
|---|---|
| `lexer.rs` | Recognize `:` and `->` |
| `ast.rs` | Store optional variable, parameter, and return annotations |
| `parser.rs` | Parse annotated declarations and functions |
| `types.rs` | Define source-level types and compatibility helpers |
| `type_checker.rs` | Resolve known declarations, infer expression facts, check contracts and return paths |
| `runtime.rs` | Enforce dynamic values at typed binding and function boundaries |
| `diagnostics.rs` | Render stable static and runtime type diagnostics |
| `lib.rs` | Orchestrate lexing, parsing, checking, then execution |

The proposed module split is a design target, not an assertion that these files already exist.

### Type and runtime boundaries

The implementation must distinguish a declared contract, the static type fact known for an expression, and the actual runtime value tag. An unannotated binding is treated as `any`; it is not permanently inferred from its initializer in 0.2.0.

Runtime guards are required when an `any` value enters an annotated binding, annotated parameter, annotated return, or another context requiring a concrete type. Binding writes and function entry/return checks must be centralized to prevent bypasses.

Arrays remain heterogeneous. The only array annotation is `array`; indexed reads have static type `any`. Generic collection types are deferred.

### Architectural constraints

- Keep the current single-crate modular architecture; do not split crates without a demonstrated dependency boundary.
- Keep the type checker independent of runtime side effects.
- Preserve source locations through AST and checker diagnostics.
- Do not add unsafe Rust without a measured requirement and safety review.
- Preserve short-circuit semantics and 0.1.x dynamic behavior for unannotated code.
- Do not claim a target module or feature exists until implemented and tested.

## Deferred architecture

HIR/MIR, bytecode, optimizer, VM, LLVM/native backend, WASM, package manager, LSP, and C/C++ FFI remain future work. They are not prerequisites for gradual typing in 0.2.0.
