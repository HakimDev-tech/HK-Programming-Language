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

The 0.2.0 target is gradual typing, not a general-purpose inference engine. Unannotated bindings remain dynamically typed (`any`). Alpha.1 syntax, alpha.2 static checking, and alpha.3 runtime contracts are implemented on feature branches; alpha.4 stabilization is validated. Alpha.5 implementation and acceptance gates are complete on its feature branch, focused on precise diagnostics at dynamic boundaries; PR #10 remains unmerged. Alpha.6 implementation is in progress on its feature branch, with scope and loop-analysis changes and focused regression coverage; exact-head CI and fuzz validation remain pending. These changes are not merged or stable releases. See [HK-TYPING-SPEC.md](HK-TYPING-SPEC.md), [HK-ALPHA5-SPEC.md](HK-ALPHA5-SPEC.md), [HK-ALPHA6-SPEC.md](HK-ALPHA6-SPEC.md), and [HK-IMPLEMENTATION-PLAN.md](HK-IMPLEMENTATION-PLAN.md).
