# HK 0.1.0 — Architecture

## Decision

The 0.1.0 implementation is a Rust modular monolith in one Cargo package. The implementation is split into explicit Rust modules: lexer, ast, parser, runtime, and diagnostics; the CLI remains in main.rs. This avoids premature crate boundaries while preserving separable responsibilities. Extract modules into independent workspace crates only after dependency contracts are proven.

TypeScript is reserved for future developer tooling (playground, editor integration, website). C++ is reserved for optional native interoperability or measured low-level components. Neither is a dependency of the 0.1.0 compiler.

## Execution pipeline

~~~text
UTF-8 source -> Lexer -> Tokens with locations -> Parser -> AST
             -> Tree-walking interpreter -> Runtime values/scopes -> output
~~~

## Runtime

Values use a tagged Rust enum. Arrays use shared reference-counted interior mutability. Environments are lexical parent-linked scopes. A constant protects a binding from reassignment, not the mutability of an object it references.

The interpreter propagates return control flow explicitly. Invalid HK programs must return diagnostics rather than trigger Rust panics.

## Future architecture

~~~text
Source -> Lexer -> Parser -> AST -> Resolver -> HIR
      -> Progressive Type Checker -> Typed IR
      -> Optimizer -> Bytecode VM / Native backend / WASM backend
~~~

HIR, MIR, bytecode, LLVM, WASM, package manager, LSP, and C/C++ FFI are not implemented in 0.1.0.

## Engineering constraints

- Preserve source locations through the frontend and render source excerpts with caret markers.
- Bound parser nesting, expression evaluation, statement execution, function call depth, and recursive value display.
- Traverse environment parent chains and array-reference graphs iteratively to avoid host-stack growth.
- Keep language semantics independent of CLI formatting.
- Avoid unsafe Rust without a measured requirement and safety review.
- Add regression tests for fixed language bugs.
- Do not claim roadmap features are implemented until shipped and tested.
