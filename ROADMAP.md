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


**Continuation update (2026-11-19):** a staged parser slice now models/parses trait signatures and impl declarations, generic impl headers, typed method parameters/returns and bounded body token spans. Duplicate methods/parameters are rejected and the dedicated syntax fuzz target now includes this parser. Commit `eefd7dc479375a861cd799d0bf9ec25fbb6d2e44` is pending CI. Still isolated, not executable; M1.1 remains incomplete and M1.2–M8 remain unimplemented.

**Latest verified continuation:** head `d9270d6c9d6be68439aa1ddee5295325897dcec3` passed CI #842 across Linux/macOS/Windows (format, tests, Clippy, rustdoc). Added trait/impl signature parser with generic impl declarations, typed parameters/returns, opaque body spans, duplicate checks, first-self enforcement and bounded nesting. Fuzz #371 remains running. Intermediate CI failures were corrected; the parser remains isolated and is not executable compiler functionality.

**Checkpoint (2026-11-19):** isolated v0.5 syntax now additionally parses generic and nongeneric function declaration signatures (bounds, typed params, optional returns) and captures function-body spans, rejecting duplicate function/parameter names and bounded nesting violations. Fuzz target covers this parser. Code head `bb547f7127499002c634dd30be8938c33af49eb9`; CI #858 passed Linux/macOS/Windows. Fuzz #378 remains in progress. No feature is compiler-integrated; M1.1 incomplete, M1.2–M8 outstanding.

**Latest checkpoint:** staged struct literal parser now captures field expression spans, rejects duplicate/empty fields and handles nested delimiters with transactional cursor rollback; the fuzz target exercises it. Head `ffd1ea40eabdbf4c6b04abef2abfa2fb6afc9af3`, CI #871 passed Linux/macOS/Windows. Fuzz #384 is still running. No compiler integration; M1.1 remains incomplete, M1.2–M8 outstanding.

**Latest exact state:** commit `4540a15e1e209c5c461e0b63a0e33a6f9efaedad`; CI #878 green Linux/macOS/Windows. Struct-literal parsing and fuzz invocation added; a fuzz target cursor bug discovered in run #384 was corrected. Fuzz #387 is in progress. No executable compiler integration; M1.1 incomplete, M1.2–M8 outstanding.

**Checkpoint:** enum constructor syntax parser added for fully-qualified unit/tuple variants and payload spans, with bounds and transactional failure. CI #890 passed Linux/macOS/Windows. Fuzz run #387 completed the existing source, syntax, module and bytecode campaigns; #391 on the enum-parser target is still running. This is isolated syntax; M1.1 incomplete and M1.2–M8 outstanding.

**Checkpoint:** match statement parser added for qualified enum patterns, tuple bindings, wildcard and block arms; duplicates and wildcard ordering checked; bounded and transactional. Head `d628663067e785cc53ce7658ee0cdccc57f311fa`, CI #904 green Linux/macOS/Windows. Fuzz #398 running. This is syntax-only; semantic exhaustiveness and compiler integration remain unimplemented, M1.1 incomplete and M1.2–M8 outstanding.

**Fuzz checkpoint:** #398 successfully completed the full configured sequence: HK source/interpreter, v0.5 syntax target + campaign (including match parser), module resolver, bytecode verifier + malformed-bytecode campaign. CI #904 green. Code head `d628663067e785cc53ce7658ee0cdccc57f311fa`. Still isolated parser, no semantic exhaustiveness or executable integration; M1.1 incomplete, M1.2–M8 outstanding.