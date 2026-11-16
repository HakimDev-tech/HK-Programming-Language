# HK Architecture

## Current baseline: HK 0.3.0

HK is a Rust modular monolith in one Cargo package. The interpreter and gradual-typing implementation are separated into focused modules:

| Module | Responsibility |
|---|---|
| `lexer.rs` | Tokenization, source positions, and language keywords |
| `ast.rs` | Expression/statement representation, type annotations, imports, and exports |
| `parser.rs` | Parsing declarations, control flow, expressions, imports, and exports |
| `types.rs` | Source-level type representation and compatibility relation |
| `type_checker.rs` | Lexical type scopes, static expression checks, function signatures, and return-path analysis |
| `runtime.rs` | Reference tree-walking execution, lexical environments, contracts, import aliases, and native math functions |
| `module_loader.rs` | Filesystem resolution, graph discovery, import/export linking, cycle validation, and dependency-first execution |
| `diagnostics.rs` | Structured diagnostics with source excerpts and caret positions |
| `lib.rs` | Public source and filesystem execution APIs |
| `main.rs` | CLI entry point |

The crate remains intentionally unified. Splitting it into multiple crates is deferred until a real dependency or compilation boundary justifies that cost.

## Execution pipelines

### Current execution baseline (HK 0.3.0)

~~~text
UTF-8 source -> Lexer -> Parser -> AST -> Type Checker (when annotated) -> Tree-walking Runtime -> output
~~~

### Filesystem API (`hk::run_path`, HK 0.3.0)

~~~text
Entry path
  -> canonicalize entry
  -> recursively discover and parse reachable modules
  -> validate module declarations and import/export links
  -> reject dependency cycles
  -> type-check every module
  -> allocate isolated module environments
  -> link read-only import aliases
  -> initialize dependencies before importers
  -> aggregate output
~~~

The filesystem loader completes discovery, link validation, and static checking before executing any module body. A module graph and its environments are scoped to one `run_path` call.

## Gradual-typing model

The source-level types are `number`, `string`, `bool`, `null`, `array`, and `any`. An omitted annotation remains dynamic; it is not inferred into a permanent binding type.

The checker distinguishes declared binding contracts, static expression facts, and runtime value tags. Runtime contracts are stored on bindings and function boundaries rather than on values or array objects. Arrays remain heterogeneous, and indexed reads are dynamically typed.

Direct named-function calls can use signatures visible at the current source position. Function aliases and unresolved forward calls remain dynamic, while runtime parameter and return contracts stay attached to the actual function value.

## Module architecture (0.3.0)

The normative contract is [HK-MODULES-SPEC.md](HK-MODULES-SPEC.md).

- Each filesystem module has an independent root lexical environment.
- Imports are named, explicit, and restricted to the module header.
- Exports are explicit top-level variables and named functions.
- Relative paths resolve from the importing module's directory and are canonicalized for identity.
- The loader caches each canonical module once per program execution.
- Dependencies initialize in deterministic depth-first, source-import order.
- Cycles are rejected before any module body executes.
- Import aliases are read-only live references to exported bindings.
- Imported functions keep their defining module environment alive.
- The `std:` namespace is resolved by an explicit registry; `std:math` is the first module.
- Imported symbols are `any` to the 0.3.0 static checker; runtime contracts remain attached to exported bindings and functions.

The runtime stores imported aliases as weak references to the target module environment. The loader owns strong references to all module environments for the duration of the run, avoiding copied snapshots and preserving exported mutable-binding identity. Importers cannot assign through an alias, while a function executing in its defining module can mutate its own exported `let` binding.

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

The interpreter propagates return control flow explicitly. Parser/runtime depth limits, bounded function recursion, iterative environment traversal, iterative array-cycle checks, and bounded module graph traversal are part of the robustness baseline. Module source is read as UTF-8. Filesystem imports are not network requests and do not resolve third-party packages.

## Architectural constraints

- Keep the type checker independent of runtime side effects.
- Preserve source locations and source identity through every module.
- Do not add unsafe Rust without a measured requirement and safety review.
- Preserve dynamic behavior for unannotated code.
- Do not flatten modules into a shared global environment.
- Do not execute module bodies during graph discovery or link validation.
- Do not claim bytecode, optimization, native compilation, or performance characteristics that are not implemented and benchmarked.

## Proposed HK 0.4.0 architecture

The design baseline is [HK-BYTECODE-VM-SPEC.md](HK-BYTECODE-VM-SPEC.md), with milestones and release gates in [HK-0.4-IMPLEMENTATION-PLAN.md](HK-0.4-IMPLEMENTATION-PLAN.md). This is a proposal only; no bytecode compiler or VM implementation is part of the current `main` baseline.

The proposed pipeline is source -> lexer -> parser -> AST -> gradual type checker -> AST-to-bytecode compiler -> mandatory bytecode verifier -> versioned in-memory bytecode program -> stack VM. The tree-walking runtime remains the reference engine during migration and differential validation. The public `hk::run` and `hk::run_path` signatures are preserved; they switch to VM execution only after all semantic and quality gates pass.

The bytecode is versioned in memory but is not a stable serialized artifact in 0.4.0. Persistent bytecode files, external bytecode loading, JIT, native compilation, WASM, and FFI remain deferred.
