# HK Programming Language

HK is an experimental general-purpose programming language implemented in Rust. It provides a tree-walking interpreter with gradual typing and a file-based module system.

> **Status:** HK 0.2.0 is integrated into `main`. HK 0.3.0 implementation is being developed and validated on `feat/hk-v0.3.0-modules-stdlib`; it is not yet merged or published. HK remains an interpreter, not an optimizing or native compiler.

## Language capabilities

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
- Explicit named imports and exports in the 0.3.0 development branch
- Isolated file-level module scopes, canonical relative-path resolution, dependency-first initialization, and cycle rejection
- Initial standard-library module `std:math` in the 0.3.0 development branch

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

## Modules (0.3.0 development branch)

Imports are explicit, named, and declared at the beginning of a module:

~~~hk
import { add as increment, count } from "./counter.hk";
import { sqrt, max } from "std:math";

print(increment(2));
print(count);
print(sqrt(81));
print(max(12, 18));
~~~

A module exports only declarations marked with `export`. Imported names are immutable aliases; exported mutable bindings remain live, and exported functions retain their defining module environment. Modules initialize once per `run_path` execution, dependencies initialize first, and cycles are rejected before module bodies execute.

Run a filesystem-backed program with the CLI:

~~~bash
cargo run -p hk -- run examples/modules/main.hk
~~~

The Rust library exposes `hk::run_path(path)` for filesystem-backed programs. The existing `hk::run(source_name, source)` API remains suitable for single-file source strings; it reports an error if the source contains filesystem imports.

See [HK-MODULES-SPEC.md](HK-MODULES-SPEC.md) for normative semantics.

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
cargo run -p hk -- run examples/modules/main.hk
~~~

Install locally:

~~~bash
cargo install --path compiler/crates/hk
hk run examples/hello.hk
hk run examples/modules/main.hk
~~~

## Documentation

- [HK-LANGUAGE-SPEC.md](HK-LANGUAGE-SPEC.md) — original 0.1.x language behavior
- [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md) — normative 0.2.0 gradual-typing specification
- [HK-MODULES-SPEC.md](HK-MODULES-SPEC.md) — normative 0.3.0 module and standard-library specification
- [HK-ARCHITECTURE.md](HK-ARCHITECTURE.md) — implementation architecture
- [HK-IMPLEMENTATION-PLAN.md](HK-IMPLEMENTATION-PLAN.md) — 0.2.0 finalization
- [HK-0.3-IMPLEMENTATION-PLAN.md](HK-0.3-IMPLEMENTATION-PLAN.md) — 0.3.0 implementation and release gates
- [ROADMAP.md](ROADMAP.md)
- [CHANGELOG.md](CHANGELOG.md)
- [examples/](examples/)

## Runtime limits

The parser rejects nesting beyond 128 guarded grammar steps. Runtime expression and statement nesting are capped at 128; user-function calls are capped at 64; array display depth is capped at 64. The module loader caps dependency depth at 128, reachable modules at 1,024 per run, each module source at 1 MiB, and the aggregate module graph at 16 MiB. Cyclic array references and cyclic module dependencies are rejected. These are interpreter constraints, not guarantees of an optimizing/native compiler.

## Scope

HK 0.3.0 does not include package management, third-party dependency resolution, cyclic imports, re-exports, default exports, namespace imports, bytecode, native compilation, WebAssembly, concurrency, or native FFI. Imported names are statically treated as `any` in this release; runtime contracts on exported values and functions remain authoritative.

## License

MIT. See [LICENSE](LICENSE).
