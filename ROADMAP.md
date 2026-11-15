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

HK 0.2.0 is integrated into `main` by squash merge commit `0ab7d567c04beb30a603d94fab34b908aa71391a`. The validated PR head `38fca78f3ebc05f016a828522de080569f4ef74e` passed formatting, workspace tests, Clippy, rustdoc, Linux/Windows/macOS CI, and 1,190,041 fuzz executions. The implementation and crate version are complete; Git tag and GitHub Release publication remain outstanding.

The 0.2.0 technical release gate is satisfied: all acceptance criteria in [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md), the finalization checklist in [HK-IMPLEMENTATION-PLAN.md](HK-IMPLEMENTATION-PLAN.md), exact-commit CI on Linux/Windows/macOS, and dedicated fuzzing passed.

## Deferred

Generics, unions, optional types, typed collection elements, flow-sensitive narrowing, modules, closures, bytecode, VM, native compilation, WASM, FFI, package management, LSP, formatter, and debugger are not part of 0.2.0.
