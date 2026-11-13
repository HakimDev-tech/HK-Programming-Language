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
- Optional gradual type annotations (`number`, `string`, `bool`, `null`, `array`, `any`) on the 0.2.0 feature branch
- Static type checking before execution and runtime contract checks at typed boundaries
- Modular lexer, AST, parser, type checker, runtime, and diagnostic layers
- Source excerpts and caret markers in diagnostics
- Bounded parser/runtime nesting and iterative environment lookup
- Duplicate parameter validation, controlled recursion limit, and cyclic-array rejection
- CLI: hk run, hk version, hk help

HK 0.2.0 gradual typing has its syntax, static checker, and runtime contracts implemented on feature branches. Alpha.4 stabilization is validated; Alpha.5 boundary conformance and diagnostic provenance are implemented and validated on the feature branch. The work remains unmerged and is not a stable release. See [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md), [HK-ALPHA5-SPEC.md](HK-ALPHA5-SPEC.md), and [HK-IMPLEMENTATION-PLAN.md](HK-IMPLEMENTATION-PLAN.md).

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

- HK-LANGUAGE-SPEC.md (0.1.x behavior)
- HK-TYPING-SPEC.md (0.2.0 gradual typing specification and implementation status)
- HK-ALPHA5-SPEC.md (Alpha.5 boundary conformance and diagnostic acceptance criteria)
- HK-ARCHITECTURE.md
- HK-IMPLEMENTATION-PLAN.md
- ROADMAP.md
- examples/

## Gradual typing example (0.2.0 alpha)

The 0.2.0 feature branch supports optional annotations and runtime contracts:

~~~hk
fn double(value: number) -> number {
    return value * 2;
}

let input = 21;                 // dynamic binding
let checked: number = input;    // runtime-checked boundary
let result: number = double(checked);
let operation = double;         // aliases preserve function contracts
print(operation(result));       // 84
~~~

Run it from the feature branch with `cargo run -p hk -- run examples/gradual_typing.hk`. This is alpha functionality, not a stable 0.2.0 release.

## Runtime limits

The parser rejects nesting beyond 128 guarded grammar steps. Runtime expression and statement nesting are capped at 128; user-function calls are capped at 64; array display depth is capped at 64. Cyclic array references are rejected before mutation, preventing reference-counting leaks. Environment lookup and array graph checks are iterative. These are explicit 0.1.x interpreter constraints, not guarantees of an optimizing/native compiler.

## Scope

The 0.1.x baseline is dynamically typed. The 0.2.0 gradual-typing work is still in alpha and has not been merged as a stable release. Modules, package management, bytecode, native compilation, WebAssembly, concurrency, and native FFI remain out of scope for 0.1.x.

## License

MIT. See LICENSE.
