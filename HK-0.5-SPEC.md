# HK 0.5.0 — Advanced Language Features (Design Draft)

**Status:** design draft; no source-language or runtime changes are authorized by this document alone.
**Baseline:** HK 0.4.0, merged at `c47a06cec1120edd33114cce1857c40f10b7f333`.
**Purpose:** introduce nominal aggregate types and statically checked abstraction without weakening HK's gradual-typing guarantees or the verified-bytecode boundary.

## 1. Release objective

HK 0.5.0 adds a coherent, minimal object-and-abstraction model:

1. nominal structs with named fields and methods;
2. enums with explicitly named variants and payloads;
3. traits as compile-time interfaces, implemented explicitly;
4. bounded generic functions, structs, and enums;
5. inference for generic call sites where constraints determine a unique solution.

Closures are already supported by the 0.4 VM. This release must preserve and broaden their compatibility with aggregate values and generic functions; it must not describe closures as a newly introduced feature.

This is not a general object-oriented runtime, a dynamic dispatch system, or a replacement of the gradual type model.

## 2. Normative design principles

- **Nominal identity:** two separately declared structs/enums are distinct even if their fields/variants match.
- **Gradual compatibility:** existing unannotated programs retain their 0.4 behavior. New declarations are statically checked; dynamic values crossing typed boundaries are guarded at runtime where a concrete check is possible.
- **No implicit structural conformance:** a type implements a trait only through an explicit `impl Trait for Type` declaration.
- **No hidden allocation model:** struct/enum values follow the VM's existing tagged-value and shared-identity model. No unsafe Rust is introduced.
- **Verifier-first:** new bytecode instructions, constants, type descriptors, and call forms must be represented in the bytecode format and validated by the mandatory verifier before execution.
- **Determinism:** type resolution, trait selection, method lookup, diagnostics, and module linking must not depend on hash-map iteration order.
- **Source fidelity:** every declaration, field, variant, method, and generic constraint retains source identity and spans for diagnostics.
- **No silent language drift:** syntax and semantics are specified before implementation; unresolved choices are recorded as blockers, not guessed in the compiler.

## 3. Proposed source syntax

The following syntax is the design target and must be validated against the existing parser conventions before it becomes normative.

### 3.1 Structs

```hk
struct Point {
    x: number,
    y: number,
}

let origin: Point = Point { x: 0, y: 0 };
let shifted = Point { x: origin.x + 1, y: origin.y };
```

- Fields are declared in source order and initialized exactly once in a struct literal.
- Missing, duplicate, and unknown fields are compile-time errors.
- Field access uses `value.field`.
- Field assignment uses `value.field = expression` only when the value is a mutable struct instance and the field is not declared immutable by the final syntax decision.
- Struct identity is nominal. Struct equality remains identity-based unless a future explicit equality protocol is designed.
- No implicit default constructor, inheritance, visibility system, or structural object conversion in 0.5.

### 3.2 Methods and implementations

```hk
impl Point {
    fn magnitude(self) -> number {
        return std:math.sqrt(self.x * self.x + self.y * self.y);
    }
}
```

- Methods are associated with exactly one nominal type.
- The receiver convention (`self` by value/reference) is an explicit design gate: the spec must settle ownership/aliasing behavior before implementation.
- No method overloading by return type.
- Duplicate method names within the same implementation are rejected.
- Method lookup is deterministic and must not silently fall back to arbitrary dynamic property lookup for statically known nominal values.

### 3.3 Enums

```hk
enum Result<T, E> {
    Ok(T),
    Err(E),
}

let value: Result<number, string> = Result::Ok(42);
```

- Variants are qualified by enum name.
- Unit and tuple-payload variants are in scope; named-field variants are deferred unless they can be specified without ambiguity.
- Each variant has fixed arity and declared payload types.
- Construction checks variant identity and payload types.
- Exhaustive pattern matching is a separate language feature and is not implicitly included. The 0.5 design must choose a minimal, safe inspection/matching syntax or explicitly defer destructuring and provide only construction plus tag/payload access.
- Invalid variant names and payload arity/type mismatches are rejected before execution when statically knowable.

### 3.4 Traits

```hk
trait Display {
    fn display(self) -> string;
}

impl Display for Point {
    fn display(self) -> string {
        return "Point(" + self.x + ", " + self.y + ")";
    }
}
```

- Traits define method signatures, not fields or stored state.
- Implementations are explicit and checked for missing, extra, duplicate, and incompatible methods.
- Orphan/coherence policy: an implementation may be declared only in the module that owns the trait or the implementing type. Duplicate implementations for the same trait/type pair are rejected across the reachable module graph.
- Trait objects and runtime dynamic dispatch are out of scope for 0.5 unless separately justified and specified.
- Default trait methods are deferred.
- Method dispatch for statically known types is resolved before bytecode execution.

### 3.5 Generics

```hk
fn identity<T>(value: T) -> T {
    return value;
}

fn show<T: Display>(value: T) -> string {
    return value.display();
}

struct Box<T> {
    value: T,
}
```

- Type parameters are lexically scoped to their declaration.
- Bounds use explicit trait constraints.
- Generic calls infer type arguments from argument and expected-result types when a unique solution exists; otherwise explicit type arguments are required.
- Type inference must not inspect runtime values.
- Generic constraints are checked statically; calls through `any` retain runtime checks only where meaningful.
- Recursive generic instantiation and cyclic bounds require explicit termination/resource limits.
- The implementation strategy (monomorphization versus erased generic descriptors) is an architectural decision gate. It must be benchmarked and assessed against bytecode verifier complexity, recursion, module linking, and diagnostics before selection. No strategy is pre-authorized here.

## 4. Type-system integration

The current primitive types remain: `number`, `string`, `bool`, `null`, `array`, and `any`.

0.5 adds nominal type references, generic type applications, and trait bounds. The checker must distinguish:
- a declared nominal type;
- a generic type parameter;
- a trait constraint on that parameter;
- an unknown/dynamic `any` expression;
- a runtime value tag.

Required rules:
- reject unknown types, duplicate type parameters, invalid bounds, and wrong generic arity;
- validate field initializers, method receivers, method calls, enum payloads, and trait implementations;
- preserve declaration-site and call-site source spans;
- never treat `any` as proof that a trait bound is satisfied;
- define precisely which runtime guards are possible when a dynamic value enters a concrete nominal boundary;
- retain existing gradual typing and runtime contract semantics for all pre-0.5 programs.

Unions, optional types, variance annotations, higher-kinded types, associated types, and trait objects are deferred.

## 5. Runtime and bytecode requirements

- Add explicit runtime representations for struct instances and enum instances; define clone/identity/mutation behavior.
- Keep values safe and tagged; no unchecked casts from source-provided type identifiers.
- Add bytecode operations or constant descriptors only after the instruction semantics are specified.
- Extend the verifier to validate type/descriptor indices, field layouts, enum tags/payload arity, method targets, generic metadata, and all new control-flow/stack effects.
- Malformed bytecode must be rejected before any user code executes.
- VM resource limits must cover aggregate allocation, nesting, generic instantiation, and recursive type resolution.
- The tree-walking reference engine must either gain equivalent semantics for differential tests or the exact unsupported comparison boundary must be justified and tested independently.

## 6. Modules and visibility

- Struct, enum, trait, and generic declarations follow the existing explicit export/import model.
- Imported type names are read-only symbols and preserve defining-module source identity.
- Type/trait resolution must complete before any module body executes.
- Cross-module duplicate trait implementations and coherence conflicts are rejected during graph validation.
- No package registry, third-party dependency resolution, cyclic imports, or implicit prelude is introduced.

## 7. Diagnostics

The design must allocate stable diagnostic codes and test:
- unknown/duplicate nominal declarations;
- unknown/duplicate/missing fields;
- invalid enum variant, arity, or payload;
- invalid receiver/method call;
- missing/extra/incompatible trait methods;
- conflicting or orphan trait implementation;
- unknown generic parameter, wrong generic arity, unsatisfied bound, ambiguous inference, and recursive-instantiation limit;
- dynamic value violating a nominal runtime contract.

Diagnostics must identify the most useful declaration and use site, not only report a generic type mismatch.

## 8. Explicit non-goals

- Inheritance, subclasses, virtual method tables, reflection, macros, operator overloading.
- Trait objects, runtime downcasting, dynamic dispatch.
- Pattern matching unless explicitly added by an approved spec amendment.
- Async/await, concurrency, ownership/borrow checker, lifetimes.
- Native compilation, JIT, WASM, FFI, package manager, LSP, formatter, debugger.
- Performance claims without reproducible baseline comparisons.

## 9. Design gates before implementation

Implementation is blocked until the following are decided and recorded:
1. exact grammar and ambiguity tests for declarations, literals, methods, enum constructors, and generic arguments;
2. receiver semantics and mutation/aliasing rules;
3. enum inspection and pattern-matching boundary;
4. generic compilation strategy and recursion/resource limits;
5. trait coherence and cross-module rules;
6. dynamic `any` boundary behavior;
7. bytecode representation and verifier invariants;
8. migration/compatibility policy and stable diagnostics.

## 10. Acceptance criteria

The release cannot be declared complete until:
- every normative rule has positive and negative parser/type-checker/runtime tests;
- all existing 0.1–0.4 regression suites pass unchanged unless an explicitly approved compatibility change exists;
- structs, enums, traits, and generics work in source and filesystem module execution;
- cross-module type imports and trait coherence are deterministic;
- verifier adversarial tests reject malformed type/aggregate metadata;
- source and module differential tests pass wherever both engines implement the feature;
- fuzzing covers parsing, type resolution, generic instantiation, module graph/type linking, and malformed bytecode;
- CI passes formatting, workspace tests, Clippy with warnings denied, and rustdoc on Linux, macOS, and Windows;
- benchmarks document compile/verification/runtime costs and any regressions;
- README, language spec, architecture, changelog, and release notes agree with actual implementation.

**Status:** proposal for review. No implementation claim is made by this document.
