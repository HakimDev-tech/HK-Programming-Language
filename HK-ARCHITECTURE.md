# HK Architecture

## Current baseline: HK 0.4.0

HK is a Rust modular monolith in one Cargo package. The 0.4 execution pipeline is:

~~~text
UTF-8 source -> Lexer -> Parser -> AST -> Gradual Type Checker
  -> AST-to-bytecode Compiler -> Mandatory Bytecode Verifier
  -> Versioned In-Memory Bytecode -> Stack VM -> output
~~~

The public hk::run and hk::run_path entry points execute through verified bytecode. The tree-walking runtime remains the semantic reference. Bytecode is versioned in memory, not a stable serialized artifact.

## Core subsystems

| Subsystem | Responsibility |
|---|---|
| Lexer / parser / AST | Source syntax, spans and declarations |
| Types / type checker | Gradual contracts and static checks |
| Compiler | AST-to-bytecode lowering and source maps |
| Bytecode verifier | Control flow, stack effects, scopes, captures and references |
| VM | Bounded execution, calls, closures and runtime operations |
| Runtime core | Tagged values, binding cells, environments, contracts and arrays |
| Module loader/linker | Deterministic graph discovery, validation, linking and initialization |
| Diagnostics | Stable source-aware errors |
| Public API / CLI | Source and filesystem execution |

## Compatibility invariants

- Primitive types remain number, string, bool, null, array and any.
- Omitted annotations remain dynamic.
- Values use safe tagged representations.
- Arrays and closures preserve established shared identity/capture behavior.
- Module roots remain isolated; imports are explicit, read-only live aliases.
- The bytecode verifier is mandatory on every VM path.
- Source identity and spans survive compilation and module linking.
- Existing 0.4 programs retain behavior.

## HK 0.5.0 architecture target — implementation pending

The normative feature contract is HK-0.5-SPEC.md; milestones and evidence are in HK-0.5-IMPLEMENTATION-PLAN.md.

The 0.5 target extends the front end with nominal declarations, recursive type expressions, generic parameters, trait bounds, impl blocks, enum constructors and exhaustive match patterns. A whole-graph declaration/type phase must precede module initialization. The checker resolves concrete generic applications and monomorphizes functions and nominal type descriptors under strict limits. Trait calls resolve statically; only member lookup through any uses runtime nominal lookup.

The runtime adds safe reference-counted struct instances and immutable tagged enum instances. Struct fields mutate through shared aliases; nominal equality is identity-based. Methods receive a shared self receiver. The bytecode carries verified nominal descriptors, concrete specializations, field layouts, variant tags, method targets and match dispatch tables. The verifier rejects malformed metadata before VM entry. The reference runtime is extended for differential parity.

No 0.5 feature is considered delivered until parser, checker, runtime, compiler, verifier, VM, module and test paths all support it. No unsafe Rust, unchecked source-derived indices, unbounded generic expansion, or execution before whole-program verification is permitted.
