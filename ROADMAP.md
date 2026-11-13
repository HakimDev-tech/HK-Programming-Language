# HK Roadmap

| Version | Theme | Main deliverable |
|---|---|---|
| 0.1.x | Language Core | Dynamic imperative interpreter and Rust runtime hardening |
| 0.2.0 | Gradual Typing | Optional annotations, static checking, runtime contracts, and conformance guarantees |
| 0.3.x | Modules and Standard Library | Imports and a deliberately scoped standard library |
| 0.4.x | Bytecode and VM | Versioned bytecode and virtual machine |
| 0.5.x | Advanced Language Features | Evaluate structs, enums, traits/interfaces, closures, and generics |
| 0.6.x | Native Compilation | Evaluate a native backend only after technical assessment |
| 0.7.x | Memory and Concurrency | Defined resource model and concurrency primitives |
| 0.8.x | Interoperability | WebAssembly, JavaScript host interop, C ABI and optional C++ interop |
| 0.9.x | Developer Ecosystem | Package manager, LSP, formatter, debugger, release candidates |
| 1.0.0 | Stable Release | Stable language specification and compatibility guarantees |

Each release requires acceptance criteria, regression tests, documentation, and explicit known limitations. Performance claims require reproducible benchmarks. Roadmap items are plans, not shipped capabilities.

## 0.2.0 status

The gradual-typing implementation candidate includes annotation syntax, static checking, runtime contracts, dynamic-boundary diagnostics, lexical-scope conformance, and a dedicated regression/fuzzing setup. The current integration candidate is undergoing final semantic review and documentation consolidation. It is not yet merged into `main`, published, or stable.

The 0.2.0 release gate requires all acceptance criteria in [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md), the finalization checklist in [HK-IMPLEMENTATION-PLAN.md](HK-IMPLEMENTATION-PLAN.md), exact-commit CI on Linux/Windows/macOS, and a successful dedicated fuzz run.

## Deferred

Generics, unions, optional types, typed collection elements, flow-sensitive narrowing, modules, closures, bytecode, VM, native compilation, WASM, FFI, package management, LSP, formatter, and debugger are not part of 0.2.0.
