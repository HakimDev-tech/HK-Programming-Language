# HK Programming Language

HK is an experimental general-purpose programming language implemented in Rust. The 0.1.x line provides a small dynamically typed imperative language with a tree-walking interpreter. HK 0.2.0 extends that language with gradual typing.

> **Status:** HK 0.2.0 is integrated into `main` and is the current implementation baseline. HK 0.3.0 is under design and implementation on a dedicated branch. The implementation remains a tree-walking interpreter, not an optimizing or native compiler.

## Language capabilities (0.2.0 baseline)

- `let` and `const` bindings
- Number, String, Boolean, Null, and Array values
- Arithmetic, comparison, equality, and short-circuit boolean operators
- `if` / `else`, `while`, and C-style `for`
- Named functions, positional arguments, recursion, and `return`
- Lexical block scopes
- Array indexing, mutation, and `.length`
- `print(value)`
- Optional gradual type annotations: `number`, `string`, `bool`, `null`, `array`, and `any`
- Static checking before execution and runtime contracts at typed boundaries
- Source-aware diagnostics with excerpts and caret markers
- Bounded parser/runtime nesting and function recursion
- Duplicate parameter validation and cyclic-array rejection

## Gradual typing

Annotations are optional. Unannotated bindings remain dynamic; annotated bindings and function boundaries carry runtime-enforced contracts.

~~~hk
fn double(value: number) -> number {
    return value * 2;
}

let input = 21;
let checked: number = input;
let result: number = double(checked);
let operation = double;
print(operation(result)); // 84
~~~

The source-level type vocabulary is deliberately small. Arrays remain heterogeneous, indexed reads are dynamic, and function aliases remain dynamically called unless a direct named-function signature is statically available.

## HK 0.3.0 — planned module model

The 0.3.0 design introduces explicit named imports and exports, isolated file-level module scopes, deterministic relative-path resolution, at-most-once initialization, cycle rejection, and a small `std:math` module. These are design targets, not shipped capabilities yet.

See [HK-MODULES-SPEC.md](HK-MODULES-SPEC.md) for the normative design and [HK-0.3-IMPLEMENTATION-PLAN.md](HK-0.3-IMPLEMENTATION-PLAN.md) for release gates.

## Requirements

Rust stable toolchain with Cargo.

## Build and test

~~~bash
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo run -p hk -- version
~~~

## Run a program

~~~bash
cargo run -p hk -- run examples/hello.hk
cargo run -p hk -- run examples/gradual_typing.hk
~~~

Install locally:

~~~bash
cargo install --path compiler/crates/hk
hk run examples/hello.hk
~~~

## Documentation

- [HK-LANGUAGE-SPEC.md](HK-LANGUAGE-SPEC.md) — 0.1.x language behavior
- [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md) — normative 0.2.0 gradual-typing specification
- [HK-MODULES-SPEC.md](HK-MODULES-SPEC.md) — normative 0.3.0 module and standard-library design
- [HK-ARCHITECTURE.md](HK-ARCHITECTURE.md) — implemented architecture and target module architecture
- [HK-IMPLEMENTATION-PLAN.md](HK-IMPLEMENTATION-PLAN.md) — 0.2.0 finalization
- [HK-0.3-IMPLEMENTATION-PLAN.md](HK-0.3-IMPLEMENTATION-PLAN.md) — 0.3.0 implementation and release gates
- [ROADMAP.md](ROADMAP.md)
- [CHANGELOG.md](CHANGELOG.md)
- [examples/](examples/)

## Runtime limits

The parser rejects nesting beyond 128 guarded grammar steps. Runtime expression and statement nesting are capped at 128; user-function calls are capped at 64; array display depth is capped at 64. Cyclic array references are rejected before mutation. Environment lookup and array graph checks are iterative. These are interpreter constraints, not guarantees of an optimizing/native compiler.

## Scope

The 0.2.0 implementation does not yet include modules, package management, bytecode, native compilation, WebAssembly, concurrency, or native FFI. The 0.3.0 module design does not imply these features are implemented.

## License

MIT. See [LICENSE](LICENSE).
