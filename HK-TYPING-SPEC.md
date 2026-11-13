# HK 0.2.0 — Gradual Typing Specification

**Status:** Design baseline; implementation not started  
**Target release:** 0.2.0  
**Typing model:** Gradual typing with explicit annotations and dynamic `any`  
**Implementation baseline:** Rust tree-walking interpreter from 0.1.x

This document is the normative design baseline for the 0.2.0 typing work. If implementation constraints require a semantic change, update this document and its tests before merging the implementation.

## 1. Goals and non-goals

HK 0.2.0 adds optional type annotations to variables and named-function parameters/returns. Annotated bindings and function boundaries carry enforceable contracts. Unannotated code retains 0.1.x dynamic behavior.

The release does not add generics, unions, optional types, structural typing, classes, interfaces, anonymous functions, overloads, implicit conversions, or a bytecode/native backend.

## 2. Type vocabulary

The source-level type names are:

| Type | Meaning |
|---|---|
| `number` | Finite IEEE-754 binary64 number |
| `string` | UTF-8 string |
| `bool` | Boolean |
| `null` | The single null value |
| `array` | Any existing HK array, including heterogeneous arrays |
| `any` | Dynamically typed value; a runtime check is required when crossing a typed boundary |

Function values remain runtime values but are not annotatable as a source-level type in 0.2.0.

The checker also uses internal expression knowledge for literals and operators. This does not introduce additional source-level types.

## 3. Syntax

### 3.1 Variable declarations

```ebnf
variableDecl = ("let" | "const") IDENTIFIER [ ":" type ] "=" expression ";" ;
type         = "number" | "string" | "bool" | "null" | "array" | "any" ;
```

Examples:

```hk
let count: number = 0;
const title: string = "HK";
let dynamicValue = 42;
let explicitDynamic: any = "value";
```

An omitted annotation is equivalent to an `any` binding for static checking and preserves dynamic reassignment semantics. It is not inferred into a permanently typed binding in this release.

### 3.2 Named functions

```ebnf
functionDecl = "fn" IDENTIFIER "(" [ parameters ] ")" [ "->" type ] "{" declaration* "}" ;
parameters   = parameter { "," parameter } ;
parameter    = IDENTIFIER [ ":" type ] ;
```

Examples:

```hk
fn add(a: number, b: number) -> number {
    return a + b;
}

fn log(value: any) {
    print(value);
}

fn identity(value) {
    return value;
}
```

Omitted parameter annotations are `any`. An omitted return annotation preserves 0.1.x behavior and imposes no declared return contract. A declared return type applies to every explicit and implicit return path.

### 3.3 Lexical tokens

The lexer must recognize colon (`:`) and arrow (`->`) without changing the interpretation of existing punctuation or minus expressions. The arrow is a single token and must be recognized before a standalone minus token.

## 4. Static type model

The checker uses the following internal type set:

```text
Number | String | Bool | Null | Array | Any
```

A declared annotation is a contract on a binding or function boundary. The checker must distinguish:
1. Declared type: the contract written in source.
2. Static expression type: what the checker can prove at a source location.
3. Runtime value type: the tag of the actual value.

These are not interchangeable.

### 4.1 Compatibility relation

A source expression of known type `S` is statically compatible with expected type `T` when:
- `S = T`; or
- `S = Any`, in which case a runtime guard is required.

A known concrete type different from the expected concrete type is a static error. There are no implicit conversions. `null` is not assignable to any type other than `null` or `any`.

Values of any type may flow into an `any` destination.

### 4.2 Binding semantics

- An annotated variable's initializer must satisfy its declared type.
- Every later assignment to an annotated mutable binding must satisfy the same contract.
- The check applies to initialization and assignment, regardless of which runtime path performs the write.
- An unannotated binding is dynamically typed and may change value type, as in 0.1.x.
- `const` prevents rebinding, not mutation of an array referenced by the binding.
- Shadowing creates a distinct binding with its own declared type.
- Duplicate declarations and lexical scope rules remain those of 0.1.x.

### 4.3 Function semantics

- Function arity remains exact.
- Each annotated parameter is a contract checked at function entry.
- An argument whose static type is known to be incompatible is rejected before execution.
- An argument of static type `any` is checked at runtime before the function body can use it under the parameter contract.
- An annotated return type applies to every returned value.
- An `any` return expression requires a runtime guard.
- A known incompatible return expression is a static error.
- A function with no return annotation preserves the 0.1.x implicit-null behavior.
- A function annotated `-> null` may fall through because its implicit result is null.
- A function annotated with any other concrete type must not have a reachable fallthrough path.

## 5. Expression typing

| Expression | Static result / constraint |
|---|---|
| Numeric literal | `number` |
| String literal | `string` |
| Boolean literal | `bool` |
| `null` literal | `null` |
| Array literal | `array`; elements remain heterogeneous |
| Variable read | Declared type, or `any` for unannotated binding |
| Assignment | Type of assigned value, subject to binding contract |
| Unary `-` | Requires `number`; result `number` |
| Unary `!` | Requires `bool`; result `bool` |
| Arithmetic `-`, `*`, `/`, `%` | Requires two `number` operands; result `number` |
| `+` | Two numbers -> `number`; two strings -> `string`; known incompatible pair -> error |
| Ordering comparisons | Requires two `number` operands; result `bool` |
| Equality | Result `bool`; preserves existing runtime equality semantics |
| `&&`, `||` | Require `bool`; result `bool`; short-circuit preserved |
| Array index | Index must be `number`; result `any` |
| Array `.length` | `number` |
| Known named-function call | Declared return type, otherwise `any` |
| Call through dynamic value | `any`; runtime retains callability and arity checks |

For an operator with an `any` operand, the checker inserts no conversion. The runtime operator remains responsible for checking the actual operand tags and reporting the existing operator error if the operation is invalid. When an `any` value is used where a concrete type is required (for example, a typed binding, parameter, return, or condition), a contract guard is required.

For `+`, a known `number` plus known `string` is rejected statically. If either side is `any`, the expression remains dynamic and the existing runtime operator rules decide whether the operation is valid.

## 6. Arrays

Arrays remain ordered, mutable, and heterogeneous. The only annotatable array type in 0.2.0 is `array`.

```hk
let values: array = [10, "hello", true];
values[0] = "updated";
let first: number = values[0];
```

The first two statements are valid. The third requires a runtime check because indexed reads have static type `any`. Generic forms such as `array<number>` are deferred.

The checker does not infer element types, track array contents through aliases, or promise that a particular index retains a type.

## 7. Conditions, short-circuiting, and name resolution

Conditions of `if`, `while`, and a present `for` condition require `bool`. A known non-boolean type is rejected statically. An `any` condition receives a runtime Boolean check.

The checker must preserve left-to-right short-circuit semantics of `&&` and `||`. It must not introduce eager evaluation or make runtime-only name failures observable in a branch that is skipped.

The checker resolves declarations and function signatures that are available under the existing sequential declaration model, including a function's own signature for recursion. It must not introduce forward-reference or mutual-recursion semantics absent from 0.1.x. Runtime remains responsible for dynamic/unresolved names in paths where static proof is not available. Existing duplicate-declaration and use-before-declaration behavior must remain intact.

## 8. Return-path analysis

A function with a concrete declared return type other than `null` must return on every path that can complete normally.

The minimum conservative control-flow rules are:
- An explicit `return` terminates the current path.
- A block guarantees a return if its sequential analysis proves that no normal path reaches its end.
- An `if/else` guarantees a return only when both branches guarantee a return.
- An `if` without `else` does not guarantee a return.
- Loops are not assumed to execute and do not by themselves guarantee a return.
- An infinite loop is not proven in 0.2.0.
- A function annotated `-> null` may fall through.
- A function without a return annotation keeps 0.1.x semantics.

This is intentionally conservative; no sophisticated theorem proving or constant-condition reachability analysis is required.

## 9. Runtime contract enforcement

Runtime guards are mandatory at every boundary where a dynamic value enters a concrete contract:
- initialization of an annotated binding;
- assignment to an annotated binding;
- entry into an annotated function parameter;
- return from a function with a declared return type;
- use of an `any` value in a context requiring a concrete type, including Boolean conditions and typed operator operands where the checker cannot prove the type.

A failed guard returns a normal HK diagnostic, never a Rust panic. A guard must run before the invalid value becomes observable through the contracted binding or function body.

The implementation must centralize binding writes and function-boundary checks so no mutation path bypasses a contract. Existing array element mutation remains heterogeneous and is not governed by the array variable's `array` annotation beyond requiring the receiver to be an array.

## 10. Pipeline and error policy

The execution pipeline is:

```text
UTF-8 source -> Lexer -> Parser -> AST -> Type Checker -> Runtime
```

No program execution or output occurs if lexing, parsing, or static type checking fails. Static diagnostics are therefore deterministic and precede runtime side effects.

The checker must not reject a program solely because it cannot prove the type of an `any` expression; it must arrange the appropriate runtime guard at a concrete typed boundary.

## 11. Diagnostics

Proposed type diagnostic codes:

| Code | Meaning |
|---|---|
| E3001 | Incompatible variable initializer |
| E3002 | Incompatible assignment |
| E3003 | Incompatible function argument |
| E3004 | Incompatible function return |
| E3005 | Incompatible operator operand |
| E3006 | Condition is not Boolean |
| E3007 | Incompatible value for expected type |
| E3008 | Runtime type contract violation |
| E3009 | Typed function may fall through without returning |

Messages must state the expected and actual types where known and preserve source name, line, column, source excerpt, and caret. Codes are reserved for the 0.2.0 implementation and must be kept stable within the release candidate series.

## 12. Compatibility guarantees

The following 0.1.x behavior must remain unchanged for programs without annotations:
- dynamic variable reassignment;
- heterogeneous arrays and mutation;
- lexical scopes and shadowing;
- function arity, recursion, and implicit null return;
- operator semantics and runtime operand checks;
- Boolean-only conditions;
- short-circuit evaluation;
- array identity equality;
- existing diagnostics and safety limits, except where a clearly documented bug fix is required.

The type checker must not silently add static errors to previously valid unannotated programs. Any intentional compatibility exception requires a specification amendment, a regression test, and a changelog entry.

## 13. Acceptance criteria

A 0.2.0 release is acceptable only when all of the following hold:

1. The lexer recognizes type punctuation and arrow syntax without regressions.
2. The parser accepts every documented annotation form and rejects malformed forms with controlled diagnostics.
3. All six source-level types are recognized.
4. Annotated variable initialization and reassignment enforce their declared contracts.
5. Annotated function parameters and returns enforce their contracts.
6. Known concrete incompatibilities are rejected before runtime execution.
7. Dynamic values crossing typed boundaries are checked at runtime.
8. Return-path analysis rejects reachable fallthrough for concrete non-null return types.
9. Arrays remain heterogeneous and indexed reads remain dynamically typed.
10. Short-circuit behavior and lexical scope behavior remain intact.
11. The complete 0.1.x regression suite passes without weakening or deleting tests.
12. Malformed/generated source and type-checker inputs do not cause uncontrolled panics.
13. Formatting, tests, Clippy with warnings denied, rustdoc with warnings denied, Linux/Windows/macOS CI, and dedicated fuzzing pass.
14. Documentation and examples accurately describe shipped behavior and known limitations.

## 14. Deferred work

The following remain outside 0.2.0: generic and union types, optional types, type aliases, interfaces/traits, structs/classes, typed collection elements, flow-sensitive narrowing, global type inference, function types, anonymous functions, closures, modules, bytecode, native compilation, WASM, and FFI.
