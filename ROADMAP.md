# HK Roadmap

| Version | Theme | Main deliverable |
|---|---|---|
| 0.1.x | Language Core | Dynamic imperative interpreter and Rust runtime hardening |
| 0.2.0 | Gradual Typing | Optional annotations, static checking, runtime contracts, and conformance guarantees |
| 0.3.0 | Modules and Standard Library | Isolated modules, named imports/exports, deterministic loading, and `std:math` |
| 0.4.0 | Bytecode and VM | Verified AST-to-bytecode compiler and stack VM |
| 0.5.0 | Advanced Language Features | Nominal structs/enums, traits, and bounded generics (design under review) |
| 0.6.x | Native Compilation | Evaluate a native backend only after technical assessment |
| 0.7.x | Memory and Concurrency | Defined resource model and concurrency primitives |
| 0.8.x | Interoperability | WebAssembly, JavaScript host interop, C ABI and optional C++ interop |
| 0.9.x | Developer Ecosystem | Package manager, LSP, formatter, debugger, release candidates |
| 1.0.0 | Stable Release | Stable language specification and compatibility guarantees |

Each release requires acceptance criteria, regression tests, documentation, and explicit known limitations. Performance claims require reproducible benchmarks. Roadmap items are plans, not shipped capabilities.

## 0.2.0 status

HK 0.2.0 is integrated into `main` by squash merge commit `0ab7d567c04beb30a603d94fab34b908aa71391a`. Its implementation and crate version are complete. Historical tag and GitHub Release publication are not asserted here.

## 0.3.0 status

HK 0.3.0 is integrated into `main` by squash merge commit `644cee85f839764b146c024ab04972861bd677fe` (PR [#14](https://github.com/HakimDev-tech/HK-Programming-Language/pull/14)). Its module semantics, implementation, and cross-platform validation are recorded in [HK-MODULES-SPEC.md](HK-MODULES-SPEC.md) and [HK-0.3-IMPLEMENTATION-PLAN.md](HK-0.3-IMPLEMENTATION-PLAN.md). Historical tag/release publication is not asserted here.

## 0.4.0 status

HK 0.4.0 is merged into `main` at `c47a06cec1120edd33114cce1857c40f10b7f333` (PR [#16](https://github.com/HakimDev-tech/HK-Programming-Language/pull/16)). The post-merge CI run passed on Ubuntu, macOS, and Windows, including formatting, workspace tests, Clippy with warnings denied, and rustdoc. The candidate also passed the dedicated fuzz workflow before merge. The VM benchmark trade-off is documented in [HK-0.4-BENCHMARKS.md](HK-0.4-BENCHMARKS.md). The Git tag and GitHub Release publication are tracked separately and must not be inferred from the merge.

## 0.5.0 status

The design phase is initiated on branch `design/hk-v0.5.0-advanced-language-features`. The proposed scope and gates are in [HK-0.5-SPEC.md](HK-0.5-SPEC.md) and [HK-0.5-IMPLEMENTATION-PLAN.md](HK-0.5-IMPLEMENTATION-PLAN.md). No 0.5 language feature is implemented yet.

## Deferred and non-goals

Native compilation, WASM, FFI, concurrency, package management, LSP, formatter, and debugger remain future roadmap items. Roadmap entries are plans, not shipped capabilities.
