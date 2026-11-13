# HK Programming Language

HK is an experimental general-purpose programming language. The 0.1.0 milestone establishes a small imperative language with a dynamic runtime and a Rust tree-walking interpreter.

> Status: early development. Not a stable language or production-ready compiler.

## Current implementation

- let and const bindings
- Number, String, Boolean, Null, Array values
- Arithmetic, comparison, equality, short-circuit boolean operators
- if / else, while, C-style for
- Named functions, positional arguments, recursion, return
- Lexical block scopes
- Array indexing, mutation, and .length
- print(value)
- Modular lexer, AST, parser, runtime, and diagnostic layers
- Source excerpts and caret markers in diagnostics
- Bounded parser/runtime nesting and iterative environment lookup
- Duplicate parameter validation, controlled recursion limit, and cyclic-array rejection
- CLI: hk run, hk version, hk help

Progressive typing is a design goal for 0.2.0. It is not implemented in 0.1.0.

## Requirements

Rust stable toolchain with Cargo.

## Build and test

~~~bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p hk -- version
~~~

## Run a program

~~~bash
cargo run -p hk -- run examples/hello.hk
~~~

Install locally:

~~~bash
cargo install --path compiler/crates/hk
hk run examples/hello.hk
~~~

## Example

~~~hk
const greeting = "Hello";
let name = "HK";
fn greet(person) { print(greeting + ", " + person + "!"); }
greet(name);
~~~

## Documentation

- HK-LANGUAGE-SPEC.md
- HK-ARCHITECTURE.md
- HK-IMPLEMENTATION-PLAN.md
- ROADMAP.md
- examples/

## Runtime limits

The parser rejects nesting beyond 128 guarded grammar steps. Runtime expression and statement nesting are capped at 128; user-function calls are capped at 64; array display depth is capped at 64. Cyclic array references are rejected before mutation, preventing reference-counting leaks. Environment lookup and array graph checks are iterative. These are explicit 0.1.x interpreter constraints, not guarantees of an optimizing/native compiler.

## Scope

0.1.0 does not include progressive/static typing, modules, package management, bytecode, native compilation, WebAssembly, concurrency, or native FFI.

## License

MIT. See LICENSE.
