# HK Language Specification — 0.1.0

**Status:** Initial implementation specification  
**Version:** 0.1.0  
**Extension:** .hk  
**Implementation:** Rust tree-walking interpreter  
**Paradigm:** Imperative  
**Typing:** Dynamic in 0.1.0; progressive typing is planned for 0.2.0.

This document defines the intended observable behavior of HK 0.1.0. It is the language contract; implementation changes must preserve it or update this document and its tests.

## 1. Goals and source model

HK 0.1.0 is a small, readable, general-purpose imperative language. It provides variables, primitive values, arrays, expressions, control flow, named functions, lexical scopes, diagnostics, and a command-line runner. It prioritizes a coherent semantic core over breadth.

Source files are UTF-8 and use .hk. Identifiers are ASCII and case-sensitive: [A-Za-z_][A-Za-z0-9_]*. Whitespace is insignificant between tokens. A line comment starts with //. Simple statements end with semicolons; compound statements and function declarations do not. Blocks use braces.

## 2. Reserved keywords

let, const, fn, return, if, else, while, for, true, false, and null are reserved.

## 3. Values and types

- Number: finite IEEE-754 binary64.
- String: UTF-8 string.
- Boolean: true or false.
- Null: the single value null.
- Array: ordered, mutable, heterogeneous sequence.
- Function: named user function or native print function.

No implicit conversions are performed. Numeric literals are decimal integers or decimals. Exponents, hexadecimal notation, numeric separators, and a distinct integer type are excluded from 0.1.0. Arithmetic and ordering require Number operands. Addition also concatenates two Strings. Division and modulo by zero are runtime errors. Numeric results must remain finite.

Strings use double quotes. Supported escapes are newline, carriage return, tab, escaped quote, and escaped backslash. Other escapes are errors. String indexing and mutation are excluded.

Conditions and logical operators require Boolean. There is no truthiness conversion. && and || short-circuit left-to-right.

Arrays are ordered, mutable, and may contain different types. Indexes are zero-based non-negative integers. Out-of-range reads/writes fail. The read-only property .length returns a Number. Array equality compares identity, not contents. A const binding cannot be reassigned, but an array referenced by a constant may be mutated. Cyclic array references are rejected at indexed assignment to avoid reference-counting leaks and unbounded display recursion.

## 4. Bindings and scopes

let name = expression; declares a mutable binding. const name = expression; declares a non-reassignable binding. Initializers are mandatory. A binding must be declared before use. Duplicate declarations in one lexical scope are errors. Inner scopes may shadow outer bindings. Assignment targets the nearest binding. Assignment to a constant or undefined name is an error.

Blocks introduce lexical scopes. A for loop has a scope covering its initializer, condition, increment, and body.

## 5. Operators and precedence

| Level | Operators | Associativity |
|---|---|---|
| 1 | calls (), indexing [], property . | left |
| 2 | unary !, unary - | right |
| 3 | *, /, % | left |
| 4 | +, - | left |
| 5 | >, >=, <, <= | left |
| 6 | ==, != | left |
| 7 | && | left |
| 8 | || | left |
| 9 | = | right |

Primitive equality compares values. Arrays and user functions compare identity.

## 6. Statements and functions

~~~hk
if (condition) { print("yes"); } else { print("no"); }

while (condition) { print("loop"); }

for (let i = 0; i < 10; i = i + 1) { print(i); }

fn add(a, b) { return a + b; }
print(add(2, 3));
~~~

The for initializer and increment are optional; both semicolon separators remain mandatory. An omitted condition is treated as true.

Functions have named parameters, lexical scope, positional arguments, exact arity, and recursion. Duplicate parameter names are syntax errors. The interpreter limits active user-function calls to 64; exceeding this limit produces a runtime diagnostic rather than overflowing the host stack. A function reaching its end returns null. A bare return returns null. Top-level return is an error. The initial implementation supports named function declarations; anonymous functions and escaping closures are deferred.

## 7. Built-ins

Only print(value) is guaranteed. It accepts exactly one argument, writes its display representation to standard output, and returns null. Strings print without quotes; arrays print as bracketed comma-separated values.

## 8. Diagnostics and CLI

Lexical, syntax, and runtime errors must not cause an uncontrolled process panic. Diagnostics include an error code, message, source name, line, and column. End-of-file errors point to the end-of-input position. Runtime errors and invalid input must be reported without uncontrolled panics.

Commands:
- hk run <file.hk>
- hk version
- hk help

Exit codes: 0 success; 1 runtime error; 2 lexical or syntax error; 64 invalid CLI usage; 74 file I/O error.

## 9. Explicit non-goals

No progressive/static typing, modules, package manager, classes, structs, generics, escaping closures, async/await, threads, break/continue, bytecode VM, JIT, LLVM, WebAssembly, or native FFI in 0.1.0.

## 10. Compatibility

Before 1.0.0, breaking changes may occur but must be documented. The 0.1.x line prioritizes bug fixes and avoids unnecessary syntax churn. A stable specification and compatibility policy are required for 1.0.0.
