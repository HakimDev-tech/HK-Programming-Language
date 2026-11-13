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

The rules for `any` are operator-specific; it is not a blanket exemption from runtime checking:

- For arithmetic operators `-`, `*`, `/`, `%`, unary `-`, ordering comparisons, and unary `!`, an `any` operand receives a runtime guard for the required operand type (`number` or `bool`) at the point that operand is evaluated.
- For `&&` and `||`, each evaluated operand must be Boolean. A dynamic right operand is checked only if short-circuit evaluation reaches it; the checker must never evaluate it eagerly.
- For `+`, `any` uses the existing runtime overload behavior: number-plus-number and string-plus-string are valid, and other runtime combinations produce the existing operator diagnostic. The checker must not guess a concrete type or insert a one-sided conversion.
- Equality accepts operands of any type and therefore requires no operand-type guard.
- An `any` array receiver or index receives a runtime guard for `array` or `number`, respectively, when used for indexing. An `any` receiver for `.length` receives an `array` guard.

Known incompatible concrete operands remain static errors. Runtime guards must preserve left-to-right evaluation and must not evaluate an expression more than once.



### 5.1 Dynamic operands and assignment expressions

An assignment expression to an annotated binding has the declared binding type as its static result type, because the assignment succeeds only after its contract has been enforced. An assignment expression to an unannotated binding has the static type of its right-hand expression. In both cases, the expression evaluates to the assigned runtime value. A failed contract check prevents the write from occurring.

A dynamic function alias does not erase the callee's runtime contracts. For example:

```hk
fn square(value: number) -> number { return value * value; }
let operation = square;
operation("wrong"); // runtime parameter-contract diagnostic
```

The call through `operation` is not statically resolved in 0.2.0, but the parameter guard still runs before the function body.

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

Conditions of `if`, `while`, and a present `for` condition require `bool`. A known non-boolean type is rejected statically. An `any` condition receives a runtime Boolean check when evaluated. An omitted `for` condition means an unconditional loop and has no condition expression to check.

The checker analyzes both syntactic operands of logical expressions for statically provable errors, but runtime guards and expression evaluation remain lazy: a right operand skipped by `&&` or `||` is not evaluated and cannot produce a runtime-only error. This preserves left-to-right short-circuit semantics without suppressing genuine static errors in source code.

The checker resolves declarations and function signatures available at the current source position under the existing sequential declaration model. A function's own signature is visible in its body for direct recursion; later declarations are not visible earlier, and mutual recursion is not introduced. A direct call to a currently known function identifier can be checked for arity and argument compatibility. If a function value is copied into an unannotated variable, passed as a value, or otherwise called through an expression whose function identity is not statically known, the call is dynamic: its static result is `any`, and runtime callability/arity checks plus the callee's own parameter/return contracts remain authoritative.

Names not resolvable by the checker remain dynamic for compatibility and are diagnosed by the existing runtime when evaluated. The checker must not turn an otherwise runtime-only unresolved-name failure into an eager error in a skipped logical operand. Existing duplicate-declaration and use-before-declaration behavior must remain intact.

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

The implementation must centralize binding writes and function-boundary checks so no mutation path bypasses a contract. The declared type metadata belongs to the binding, not to the value or array object; aliases to the same array remain independently typed bindings. Existing array element mutation remains heterogeneous and is not governed by the array variable's `array` annotation beyond requiring the receiver to be an array. Rebinding an annotated binding is checked, but mutating an element of an array does not impose an element contract.

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
