# HK 0.5.0 — Advanced Language Features Specification

**Status:** normative design baseline for implementation; implementation not yet complete.
**Baseline:** HK 0.4.0, merge commit c47a06cec1120edd33114cce1857c40f10b7f333.
**Compatibility:** existing HK 0.4 programs retain their behavior unless an explicit compatibility amendment is approved.

## 1. Release contract

HK 0.5.0 adds nominal structs with methods, nominal enums with unit and tuple-payload variants, exhaustive statement-level pattern matching, explicit traits and coherent implementations, bounded generic functions/structs/enums, and integration with modules, closures, verified bytecode and the VM. Closures already exist in 0.4; 0.5 integrates them with aggregate values and generic calls rather than claiming to introduce them.

## 2. Type and compatibility model

Primitive types remain number, string, bool, null, array and any. New types are nominal declarations and generic applications.

- Nominal identity is declaration identity, never structural shape.
- Generic applications are invariant: Box<number> is not assignable to Box<any> or Box<string>.
- There is no inheritance, subtyping, structural conversion or user-defined coercion.
- any remains the gradual escape hatch but is not evidence that a generic constraint or trait bound is satisfied.
- Concrete nominal contracts compare runtime descriptor identity, including concrete generic arguments.
- A statically known concrete value receives static member checking. A receiver typed any may use runtime field/method lookup against its actual nominal descriptor; failure is a runtime diagnostic, never silent null.
- Variance, unions, optional, higher-kinded, associated and existential types are deferred.

## 3. Structs

~~~hk
struct Point { x: number, y: number, }
let origin: Point = Point { x: 0, y: 0 };
struct Box<T> { value: T, }
let box: Box<number> = Box<number> { value: 42 };
~~~

Structs are module-top-level declarations. Fields are ordered, unique, named and typed. Construction initializes every field exactly once; missing, duplicate or unknown fields are compile-time errors when statically knowable. Generic arguments may be inferred only when the expected type or field initializers uniquely determine every argument.

A struct instance is a heap-backed, reference-counted nominal object. Cloning a runtime value clones the reference; aliases observe the same fields. Fields are mutable. Field assignment mutates the object, not the variable binding. let and const continue to control rebinding only, matching the existing array model. Struct equality is identity equality. Display is deterministic and cycle/depth bounded. Recursive struct references are allowed; allocation and formatting are bounded. No defaults, visibility modifiers, inheritance, spread construction or structural literals.

## 4. Methods and receivers

~~~hk
impl Point {
    fn translate(self, dx: number, dy: number) {
        self.x = self.x + dx;
        self.y = self.y + dy;
    }
}
impl<T> Box<T> {
    fn get(self) -> T { return self.value; }
}
~~~

An inherent impl associates methods with one nominal type. Every instance method has exactly one first parameter named self; its type is inferred and it cannot be rebound. self is a shared receiver, not a move or copy; mutating self.field changes the shared instance and is visible through aliases. There are no static functions, method overloading or method-local type parameters. Method names are unique per nominal type after substitution. Statically known calls resolve at compile time. A receiver typed any uses runtime nominal method lookup, but this does not create trait objects. Method bodies preserve their defining module environment and source identity.

## 5. Enums and match

~~~hk
enum Result<T, E> { Ok(T), Err(E), }
enum Direction { North, South, East, West }
let result: Result<number, string> = Result<number, string>::Ok(42);
let north = Direction::North;
match result {
    Result<number, string>::Ok(value) => { print(value); }
    Result<number, string>::Err(message) => { print(message); }
}
~~~

Enums are module-top-level and nominal. Variants are unique and either unit-like or fixed-arity tuple payloads; named-field variants are deferred. Payload variants require exact arity. Generic arguments may be inferred only from a unique expected type or payload solution. Enum values are immutable tagged values; payload aliasing follows contained-value semantics. Enum equality is identity equality.

match is a statement, not an expression. Each arm is a fully qualified enum variant pattern followed by => and a block. Payload names bind positional values; unit variants have no binders. A wildcard arm is allowed once and must be last. Without a wildcard, every variant must appear exactly once. Duplicate, missing, unknown or wrong-arity patterns are compile-time errors for statically known enums. Matching any requires a wildcard arm; dynamic inspection fails safely if the runtime value is not the expected enum. Each arm has a lexical scope, bindings are immutable and there is no fallthrough. Guards, nested/or-patterns and match expressions are deferred.

## 6. Traits and coherence

~~~hk
trait Display { fn display(self) -> string; }
impl Display for Point {
    fn display(self) -> string {
        return "Point(" + self.x + ", " + self.y + ")";
    }
}
~~~

Traits declare instance method signatures only; every method has self first. No fields, associated types/constants, default methods or trait inheritance. Implementations provide every method exactly once with compatible signatures; missing, extra or duplicate methods are errors. At most one implementation exists for each trait/type pair across the reachable module graph. Orphan rule: an impl is legal only in the module declaring the trait or the module declaring the type. Trait resolution is static for concrete types and generic parameters with satisfied bounds. No trait objects or runtime trait dispatch. Inherent and trait methods with the same name on one type are rejected. Generic trait implementations are supported with explicit bounds. All coherence checks finish before module initialization.

## 7. Generics

~~~hk
fn identity<T>(value: T) -> T { return value; }
fn render<T: Display>(value: T) -> string { return value.display(); }
let n = identity::<number>(42);
~~~

Type parameters are lexically scoped and unique per declaration. Bounds are one or more trait constraints separated by +. Generic functions, structs, enums, inherent impls and trait impls are supported. Type arguments are invariant. No specialization, defaults, const generics or higher-kinded parameters.

Function arguments are inferred by unifying declared parameter types with statically known argument types; a concrete expected return type may add constraints. Inference is deterministic and never inspects runtime values. Zero or multiple valid solutions are errors. Explicit arguments use turbofish syntax. A call through a statically any callee cannot infer arguments; explicit concrete arguments are required. Generic aggregate construction follows the same unique-solution rule.

**Compilation strategy: monomorphization.** Each concrete type-argument tuple creates a distinct function specialization/type descriptor. This preserves concrete runtime contracts and enables statically resolved trait calls without a runtime dictionary ABI. Hard limits: instantiation nesting 32; 128 specializations per generic declaration per program; 4,096 total specializations per program; type-expression nesting 64. Limit violations are deterministic compile-time diagnostics. Cycles are detected. Bytecode size and compile cost must be benchmarked.

## 8. Modules and visibility

Types, traits and impls are module-top-level. Existing export/import syntax extends to struct, enum and trait declarations. Imported type/trait names are immutable and retain defining-module identity. All reachable declarations, bounds, impls and import links are resolved before any module body executes. Duplicate declarations and cross-module coherence conflicts fail deterministically before side effects. Inherent impls may occur only in the type's defining module; trait impls obey the orphan rule. Existing relative .hk imports, dependency order, module isolation and std:math remain unchanged. No package registry, cyclic imports or implicit prelude.

## 9. Bytecode, verifier and runtime safety

Every nominal type, specialization, field layout, enum variant, method target and match dispatch table is represented in the versioned in-memory bytecode. The verifier validates descriptor indices, type argument counts, layouts, variant tags/arity, method targets, specialization references, match targets and stack effects. No VM entry occurs before complete verification. Runtime values remain tagged; source-provided IDs are never trusted as unchecked indices or pointer casts. Allocation, payload storage, formatting, recursive resolution and generic expansion are bounded. No unsafe Rust. The reference engine must implement the same semantics before differential parity is claimed. Runtime contracts compare exact nominal descriptors.

## 10. Diagnostics

Stable feature-specific diagnostic families distinguish declaration errors; field/variant construction and access errors; method/trait conformance, orphan and coherence errors; generic arity, bounds, inference and instantiation-limit errors; match exhaustiveness and binder errors; and dynamic nominal contract/member errors. Every diagnostic preserves source file, declaration/use-site span and source excerpt. Specified errors must not collapse into generic E3000/E4009.

## 11. Non-goals

Inheritance, subtyping, structural records, trait objects, dynamic trait dispatch, default trait methods, associated types, operator overloading, macros, reflection, visibility modifiers, named enum payloads, match guards, nested/or-pattern destructuring, match expressions, method-local generics, async/concurrency, ownership/borrow checking, lifetimes, native/JIT/WASM/FFI, package manager, LSP, formatter and debugger.

## 12. Acceptance criteria

The release is complete only when grammar, AST, parser, checker, runtime, compiler, verifier, VM, modules and public API implement this contract; every rule has positive/negative tests; all 0.1–0.4 regressions pass; all features work in source and filesystem execution; specializations and trait resolution are deterministic/bounded; adversarial verifier tests reject malformed metadata before VM entry; reference/VM differential tests pass; fuzzing covers parser, type resolution, instantiation, module coherence and malformed bytecode; Linux/macOS/Windows CI passes formatting, workspace tests, Clippy with warnings denied and rustdoc; benchmarks report compile/verify/execute/code-size/memory costs; docs/examples/changelog match behavior; and the exact merged commit is tagged and published.

**Implementation status:** not implemented. This is the normative target, not a claim of shipped functionality.
