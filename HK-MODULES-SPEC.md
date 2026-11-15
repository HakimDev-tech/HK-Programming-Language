# HK 0.3.0 — Modules and Standard Library Specification

**Status:** Normative design baseline for HK 0.3.0. Implementation must conform to this document; semantic changes require updating this specification and its tests before merge.
**Target release:** 0.3.0
**Baseline:** HK 0.2.0 gradual-typing interpreter
**Execution model:** Tree-walking interpreter with a deterministic, file-based module loader

## 1. Goals

HK 0.3.0 introduces a small, explicit module system and the first versioned standard-library module. It must provide:

1. File-level encapsulation: each source file has its own module scope.
2. Explicit exports: only declarations marked with `export` are externally visible.
3. Explicit named imports with optional local aliases.
4. Deterministic relative-path resolution and module identity.
5. At-most-once module initialization per program execution.
6. Deterministic dependency ordering and clear cycle diagnostics.
7. Compatibility with HK 0.2.0 gradual typing and runtime contracts.
8. A small, documented standard-library surface with stable behavior.

The release does not introduce package management, third-party dependencies, namespace imports, dynamic imports, re-exports, default exports, or a new execution backend.

## 2. Source syntax

### 2.1 Export declarations

The `export` modifier is permitted only before a top-level variable or named-function declaration:

```hk
export const TAX_RATE = 0.19;

export fn total(price: number, quantity: number) -> number {
    return price * quantity;
}
```

Exported declarations retain the exact binding, mutability, function, and gradual-typing semantics they have without the modifier. Exporting does not copy a value or weaken a runtime contract.

The following are errors:
- exporting a block-local declaration;
- exporting an expression or control-flow statement;
- exporting the same public name more than once;
- exporting an unsupported declaration kind.

A module may contain private top-level declarations. They are visible inside that module only.

### 2.2 Named imports

```hk
import { total, TAX_RATE } from "./pricing.hk";
import { total as calculateTotal } from "../shared/pricing.hk";

print(calculateTotal(100, 2));
```

An import declaration:
- names one or more exported identifiers;
- may rename each imported identifier with `as`;
- must end with a semicolon;
- binds each imported name as an immutable local binding;
- cannot import the same local name twice in one module;
- must appear before every non-import top-level declaration.

Imports are lexical module bindings, not global variables. Importing a name that the target does not export is a module-link error. Importing a missing file or unsupported path is a module-resolution error.

Only named imports are supported. Namespace imports and wildcard imports are deliberately excluded.

### 2.3 Grammar additions

```ebnf
importDecl   = "import" "{" importName { "," importName } "}" "from" STRING ";" ;
importName   = IDENTIFIER [ "as" IDENTIFIER ] ;
exportDecl   = "export" ( variableDecl | functionDecl ) ;
```

The import path is a string literal. Escape handling is identical to ordinary HK strings. The path must be non-empty and use the `.hk` extension for filesystem modules.

## 3. Module identity and resolution

A program starts from one entry file. Filesystem imports are resolved relative to the directory of the importing file, never relative to the process working directory.

Resolution rules:
1. Resolve the import path against the importing module's parent directory.
2. Require the `.hk` extension.
3. Canonicalize the resolved path before assigning module identity.
4. Read UTF-8 source only.
5. Reject directories, missing files, invalid UTF-8, and unsupported extensions with a source-located diagnostic.
6. Canonical paths identify modules. Two paths resolving to the same canonical file refer to one module instance.

The initial release does not search arbitrary global paths, environment variables, package registries, or implicit index files. There is no implicit extension insertion. The entry file and filesystem modules must use the `.hk` extension. A module source is limited to 1 MiB, the reachable graph to 16 MiB of aggregate source and 1,024 files, and dependency depth to 128.

The standard-library namespace uses reserved specifiers beginning with `std:`. Filesystem paths cannot use that prefix. In 0.3.0, the supported standard module is `std:math`.

## 4. Module graph and initialization

The loader builds and validates the complete reachable dependency graph before executing user code.

- Dependencies are initialized before their importer.
- Imports are processed in source order.
- A module is initialized at most once in one top-level program execution, even if multiple modules import it.
- Repeated imports reuse the same module instance and exported binding identities. Imported names are live read-only views of those bindings, not copied snapshots.
- Cyclic dependencies are rejected before any module body executes.
- The cycle diagnostic reports the import edge that closes the cycle and a readable module chain.
- A failed module initialization aborts the program; partially initialized exports are never made observable.
- A new call to the public run API starts a fresh module graph and module cache.

This release deliberately rejects cycles instead of exposing partially initialized bindings. There is no temporal-dead-zone or live-binding behavior to reason about.

Top-level statements execute once, in dependency-first order. Their output is appended to the same output stream as the entry module. Import declarations themselves produce no output.

## 5. Module isolation and exported values

Every filesystem module has an independent top-level lexical environment. A module cannot read another module's private top-level bindings by name. It can access only names explicitly imported into its own scope.

Exported variables preserve binding identity and mutability semantics:
- exported `const` bindings cannot be reassigned by the exporting module or importers;
- exported `let` bindings remain mutable inside the exporting module; importers observe the current exported value but cannot assign through an imported name;
- imported names are immutable aliases: an importer cannot rebind the imported name;
- mutation of a referenced array remains governed by the existing 0.1.x/0.2.0 array semantics.

Exported functions retain their defining module environment for the lifetime of the module graph. Calls execute in the defining module's lexical environment, not the importer's environment. Function parameters, return contracts, and runtime guards remain enforced exactly as specified in HK-TYPING-SPEC.md.

Module-private names must not leak through function calls, imports, or accidental shared global environments. The existing `print` built-in remains available in every module.

## 6. Gradual typing across module boundaries

The module system does not introduce a new source-level type. Existing types remain `number`, `string`, `bool`, `null`, `array`, and `any`.

- Exported typed bindings retain their binding contracts.
- Exported typed functions retain parameter and return contracts.
- Imported references preserve the identity and runtime contracts of the exported values; importing must never unwrap or copy a function into a contract-free value.
- The static checker validates each module's own source and local declarations before execution.
- Imported values are treated as dynamic (`any`) by the importing module's static checker in 0.3.0. Runtime contracts on exported bindings and functions remain authoritative at the boundary.
- A missing export is a link error, not a dynamic undefined-name lookup.
- Existing unannotated code keeps 0.1.x dynamic behavior.

The deliberate static-typing limitation for imported symbols is documented and tested. Propagating full exported type interfaces into importers is deferred until a separate interface model is designed; it must not be approximated by unsound inference.

## 7. Standard library: `std:math`

The first standard-library module is imported explicitly:

```hk
import { abs, ceil, floor, max, min, round, sqrt } from "std:math";

print(sqrt(81));
print(max(12, 18));
```

The 0.3.0 API:

| Function | Arity | Contract | Semantics |
|---|---:|---|---|
| `abs(x)` | 1 | number -> number | Absolute value |
| `ceil(x)` | 1 | number -> number | Smallest integer-valued Number not less than x |
| `floor(x)` | 1 | number -> number | Largest integer-valued Number not greater than x |
| `round(x)` | 1 | number -> number | IEEE-754/Rust `f64::round` behavior (ties away from zero) |
| `sqrt(x)` | 1 | number -> number | Square root; negative input is a runtime error |
| `min(a, b)` | 2 | number, number -> number | Smaller operand |
| `max(a, b)` | 2 | number, number -> number | Larger operand |

All arguments must be finite Numbers. Arity and type violations produce normal source-located HK runtime diagnostics. Any non-finite result is rejected. The API is deterministic and has no I/O or mutation side effects.

The standard library is versioned with the language release. Functions not listed above are not part of the 0.3.0 public API.

## 8. Diagnostics

The implementation must define stable diagnostic codes for:
- malformed import/export syntax;
- imports not placed in the module header;
- invalid or unsupported module specifiers;
- missing/unreadable source files;
- duplicate local imports;
- unresolved exported names;
- duplicate export names;
- cyclic module dependencies;
- attempts to reassign imported bindings;
- standard-library arity, type, domain, and non-finite-result failures.

Diagnostics must identify the source module and line/column of the relevant import/export or call site. Existing lexical, parser, static typing, and runtime diagnostic behavior must remain intact.

## 8.1 Stable diagnostic code allocation

The module loader and initial standard library use these dedicated codes:

| Code | Meaning |
|---|---|
| `E4001` | Filesystem imports require `run_path`, or duplicate top-level module binding |
| `E4002` | Invalid module specifier, missing module, invalid entry extension, or excessive graph depth |
| `E4003` | Imported name is not exported or import linking fails |
| `E4004` | Duplicate or unsupported export declaration |
| `E4005` | Cyclic module dependency |
| `E4006` | Module source cannot be read as UTF-8 |
| `E3011` | Standard-library function arity violation |
| `E3012` | Standard-library argument type violation |
| `E3013` | Standard-library domain violation |
| `E3014` | Standard-library non-finite result |

Syntax and duplicate import-local diagnostics continue to use the parser's existing `E2000` and `E2001` codes. Existing dynamic runtime and gradual-typing contract diagnostics retain `E3000` and `E3008`.

## 9. Compatibility and non-goals

A single-file HK program without import/export declarations must behave identically to 0.2.0. Existing syntax and type semantics are unchanged.

Explicit non-goals for 0.3.0:
- package manager, dependency manifests, registries, lockfiles, version constraints;
- namespace, wildcard, dynamic, or conditional imports;
- cyclic imports, re-exports, default exports, module mutation APIs;
- implicit module discovery and global search paths;
- user-defined module loaders or plugins;
- network/filesystem APIs in the standard library;
- full static type-interface propagation across modules;
- bytecode, VM, optimization, native compilation, WASM, FFI, concurrency, closures, generics, classes, or traits.

## 10. Normative acceptance criteria

A release is not complete until all criteria below pass:

1. The grammar accepts valid imports and exports and rejects malformed placements.
2. Filesystem imports resolve relative to the importing file, independent of process working directory.
3. Canonical paths prevent duplicate module instances through alternate relative paths.
4. Importing the same dependency through multiple modules initializes it once.
5. Dependency initialization order is deterministic and tested.
6. Each module has an isolated lexical environment; private names cannot leak.
7. Named imports and aliases resolve only explicitly exported declarations.
8. Missing modules and missing exports fail before entry-module execution.
9. Cyclic graphs fail before any module body executes and report a useful chain.
10. Exported mutable/constant bindings preserve their defined semantics.
11. Exported functions execute in their defining module environment.
12. Typed exports preserve V0.2 runtime contracts across imports.
13. Imported names cannot be rebound by importer code.
14. All seven `std:math` functions obey arity, type, domain, and finite-number rules.
15. Every legacy 0.1.x and V0.2 regression test passes unchanged.
16. Parser, loader, and runtime malformed-input tests do not cause uncontrolled panics.
17. Cross-platform CI passes formatting, workspace tests, Clippy with warnings denied, and rustdoc with warnings denied on Linux, Windows, and macOS.
18. Fuzzing covers lexer/parser and module graph/path handling, including malformed import graphs.
19. README, language specification, architecture, changelog, examples, and CLI documentation match shipped behavior.
20. No package-manager, VM, compiler, or performance claims are made for this release.
