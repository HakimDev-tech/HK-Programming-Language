# HK Roadmap

| Version | Theme | Main deliverable |
|---|---|---|
| 0.1.x | Language Core | Dynamic imperative interpreter; stabilized modular Rust implementation |
| 0.2.x | Gradual Typing | Optional annotations, static checking, runtime contracts |
| 0.3.x | Modules and Standard Library | Imports and a deliberately scoped standard library |
| 0.4.x | Bytecode and VM | Versioned bytecode and virtual machine |
| 0.5.x | Advanced Language Features | Evaluate structs, enums, traits/interfaces, closures, and generics |
| 0.6.x | Native Compilation | Evaluate a native backend, likely LLVM only after technical assessment |
| 0.7.x | Memory and Concurrency | Defined resource model and concurrency primitives |
| 0.8.x | Interoperability | WebAssembly, JavaScript host interop, C ABI and optional C++ interop |
| 0.9.x | Developer Ecosystem | Package manager, LSP, formatter, debugger, release candidates |
| 1.0.0 | Stable Release | Stable language specification and compatibility guarantees |

Each release requires acceptance criteria, regression tests, documentation, and explicit known limitations. Performance claims require reproducible benchmarks. Roadmap items are plans, not shipped capabilities.

## 0.2.x scope note

The 0.2.0 target is gradual typing, not a general-purpose inference engine. Unannotated bindings remain dynamically typed (`any`). Alpha.1 syntax, alpha.2 static checking, and alpha.3 runtime contracts are implemented on feature branches; alpha.4 is a bounded stabilization pass with regression, positive, negative, dynamic-boundary, and fuzz-corpus coverage. This is not yet a stable or merged release. See [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md) and [HK-IMPLEMENTATION-PLAN.md](HK-IMPLEMENTATION-PLAN.md).
