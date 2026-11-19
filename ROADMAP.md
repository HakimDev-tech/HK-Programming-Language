# HK Roadmap

| Version | Theme | Main deliverable |
|---|---|---|
| 0.1.x | Language Core | Dynamic interpreter and runtime hardening |
| 0.2.0 | Gradual Typing | Optional annotations, static checking and runtime contracts |
| 0.3.0 | Modules and Standard Library | Isolated modules, explicit imports/exports and std:math |
| 0.4.0 | Bytecode and VM | Verified AST-to-bytecode compiler and stack VM |
| 0.5.0 | Advanced Language Features | Nominal structs/enums, exhaustive match, traits and bounded generics |
| 0.6.x | Native Compilation | Evaluate a native backend after technical assessment |
| 0.7.x | Memory and Concurrency | Defined resource model and concurrency primitives |
| 0.8.x | Interoperability | WASM, JavaScript host interop and C ABI |
| 0.9.x | Developer Ecosystem | Package manager, LSP, formatter, debugger and release candidates |
| 1.0.0 | Stable Release | Stable language specification and compatibility guarantees |

Roadmap entries are plans, not shipped capabilities. Every release requires acceptance criteria, regression tests, documentation, explicit limitations and exact-commit validation.

## 0.2.0

Integrated into main by squash merge commit 0ab7d567c04beb30a603d94fab34b908aa71391a. Historical tag/release publication is not asserted here.

## 0.3.0

Integrated into main by squash merge commit 644cee85f839764b146c024ab04972861bd677fe (PR #14). Module contract and implementation evidence are recorded in HK-MODULES-SPEC.md and HK-0.3-IMPLEMENTATION-PLAN.md.

## 0.4.0

Integrated into main at c47a06cec1120edd33114cce1857c40f10b7f333 (PR #16). Post-merge CI passed on Ubuntu, macOS and Windows. The candidate passed dedicated fuzz campaigns. Benchmark trade-offs are documented in HK-0.4-BENCHMARKS.md. The annotated tag and GitHub Release remain separate publication operations and must be verified independently.

## 0.5.0

The normative specification and implementation plan are maintained in PR #19 and carried into the implementation branch. D0 semantic decisions are closed in HK-0.5-SPEC.md. M1.1 has an isolated front-end slice in PR #21: token vocabulary, spans, recursive type-expression/bound models, initial type/bound parsers, and struct/enum declaration AST/parser (fields, generic bounds, unit/tuple variants, duplicate diagnostics). The parser now bounds generic parameter/bound lists (256) and rejects duplicate generic parameters and trait bounds; a dedicated `syntax_v05` fuzz target was added. Code head 6db4fd9b04ca211226bc16f578eab9af65438479 passes CI on Linux/macOS/Windows; fuzz run #351 is pending. This is not the complete grammar and is not integrated into executable compilation. M1.1 remains in progress; M1.2–M8 have not been implemented. No 0.5 feature is shipped.

## Deferred

Native compilation, WASM, FFI, concurrency, package management, LSP, formatter and debugger remain future roadmap items.
