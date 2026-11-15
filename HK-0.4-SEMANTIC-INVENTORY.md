# HK 0.4.0 — Semantic Inventory (M0)

**Status:** Baseline inventory, implementation gate in progress  
**Baseline commit:** `a3ffadb923fef7032fdb6fbc5983b4cf5957886a`  
**Baseline crate:** `hk 0.3.0`  
**Purpose:** Make the existing language surface explicit before introducing bytecode. This document is an implementation checklist, not a claim that the VM exists.

## 1. Baseline identity

The V0.4 implementation branch is based directly on the V0.3 `main` commit above. The crate manifest at `compiler/crates/hk/Cargo.toml` declares version `0.3.0`. The source front end is the existing lexer/parser/AST and the reference executor is `runtime.rs`.

The current public execution paths are:
- `hk::run(source_name, source)`: lex, parse, optionally type-check, then tree-walk.
- `hk::run_path(path)`: discover and validate the complete module graph, allocate/link module environments, then execute modules in dependency-first order.

The V0.4 implementation must preserve both public signatures and all V0.3 observable behavior.

## 2. Exhaustive AST inventory

Source: `compiler/crates/hk/src/ast.rs`.

### Expressions (`E`)

| Variant | Existing meaning | Bytecode obligation |
|---|---|---|
| `Lit(Lit, line, column)` | Numeric or string literal | Constant-pool load; preserve literal value and position |
| `Bool(bool, line, column)` | Boolean literal | Boolean constant |
| `Null(line, column)` | Null literal | Null constant |
| `Var(name, line, column)` | Lexical binding read | Resolve local/module/import binding |
| `Assign(name, value, line, column)` | Named binding assignment | Evaluate RHS once, enforce mutability/contract, return assigned value |
| `Unary(operator, operand, line, column)` | Unary `!` and `-` | Evaluate operand once; preserve typed diagnostic provenance |
| `Binary(left, operator, right, line, column)` | Arithmetic, comparison, equality, logical operators | Left-to-right evaluation; lazy lowering for `&&` and `||` |
| `Group(inner)` | Parenthesized expression | Preserve semantics; may lower transparently without losing source position |
| `Call(callee, arguments, line, column)` | Function/native invocation | Evaluate callee then arguments left-to-right exactly once; validate arity/contracts |
| `Array(elements, line, column)` | Array construction | Evaluate elements left-to-right; preserve reference identity |
| `Index(receiver, index, line, column)` | Array read | Evaluate receiver then index; validate integer/range |
| `SetIndex(receiver, index, value, line, column)` | Array mutation | Evaluate receiver, index, value in order; reject cycles; return assigned value |
| `Property(receiver, name, line, column)` | Property access (currently `length`) | Evaluate receiver once; preserve unsupported-property diagnostics |

Literal payloads are `Lit::Number(f64)` and `Lit::String(String)`. The lexer rejects non-finite numeric literals.

### Statements (`S`)

| Variant | Existing meaning | Bytecode obligation |
|---|---|---|
| `Import(path, names, line, column)` | Named module imports | No inline user instruction; link to export binding cells before execution |
| `Export(statement)` | Explicit top-level export wrapper | Export metadata; execute the wrapped declaration exactly once |
| `Var(name, initializer, mutable, line, column)` | Dynamic binding declaration | Allocate binding in correct lexical scope |
| `TypedVar(name, type, initializer, mutable, line, column)` | Contracted binding declaration | Preserve static checking and runtime binding contract |
| `Fn(name, parameters, body, line, column)` | Unannotated named function | Function prototype, recursion, defining environment |
| `TypedFn(name, params, return_type, body, line, column)` | Annotated named function | Parameter/return contracts and typed provenance |
| `Expr(expression)` | Expression statement | Evaluate once and discard result |
| `Block(statements)` | Explicit lexical block | Enter/exit child lexical scope |
| `If(condition, then, else?)` | Conditional | Boolean guard and correct branch scope |
| `While(condition, body)` | While loop | Re-evaluate condition each iteration; preserve scope rules |
| `For(initializer?, condition?, increment?, body)` | For loop | One loop scope; initializer → condition → body → increment order |
| `Return(value?, line, column)` | Function return | Unwind current frame and return value (or null) |

Types in the AST are `Number`, `String`, `Bool`, `Null`, `Array`, and `Any`. Function parameters carry a name and optional annotation. Imports carry imported/local names and source positions.

## 3. Runtime semantic obligations

Sources: `runtime.rs`, `type_checker.rs`, `module_loader.rs`, `types.rs`, `diagnostics.rs`.

### Values and identity
- Runtime values: number, string, boolean, null, shared mutable array, HK function, native print, and the seven `std:math` native functions.
- Numeric operations must reject non-finite results.
- Equality is value equality for primitive values, reference identity for arrays/functions, and identity equality for native function variants.
- Array rendering is cycle-aware and bounded by a maximum display depth.
- Array writes reject cyclic array references.
- Arrays remain heterogeneous; a variable contract of type `array` does not constrain element types.

### Bindings and scopes
- Lexical parent-linked scopes; explicit blocks create child scopes.
- `if` and `while` do not add synthetic scopes.
- A `for` loop owns one scope across initializer, condition, body, and increment.
- Declarations reject duplicates in the current scope.
- Constants prohibit rebinding, not mutation of referenced arrays.
- Imported aliases are read-only live references, not copied values.
- Typed binding contracts are checked at initialization and assignment; failed assignment leaves the old value intact.
- Function parameters and returns enforce contracts at call boundaries.
- Function values retain their defining environment; recursion and module-defined function behavior must survive the VM representation.

### Evaluation and control flow
- Expression and call argument evaluation is left-to-right.
- Function arguments are evaluated exactly once before parameter contract checks.
- `&&` and `||` short-circuit and return booleans.
- Return propagates through nested blocks/conditionals/loops to the current function frame.
- Dynamic regions remain dynamic; typed-expression provenance determines whether some runtime failures use typed-contract diagnostics or legacy dynamic diagnostics.

### Operations
- Unary: numeric negation and boolean not.
- Binary: numeric arithmetic, string+string or number+number addition, numeric ordering, equality/inequality, boolean logical operations.
- Division/modulo by zero are errors.
- Array index must be a non-negative integer within bounds.
- Only `Array.length` is supported; unsupported properties fail.
- `std:math`: `abs`, `ceil`, `floor`, `round`, `sqrt`, `min`, `max`.

## 4. Module-loader contract

Source: `module_loader.rs`.

The VM must reuse, not replace, the existing loader's responsibilities:
1. Entry and imported files must have `.hk` extension.
2. Relative imports resolve from the importing module's directory and are canonicalized.
3. `std:math` is resolved from the explicit built-in registry.
4. Import cycles are rejected before module execution.
5. Module count is bounded at 1,024; dependency depth at 128.
6. Each module source is bounded at 1 MiB; aggregate graph source is bounded at 16 MiB.
7. All reachable modules are parsed before execution.
8. Duplicate exports, duplicate module bindings, missing exports, and invalid links are rejected before any module body runs.
9. Every module is type-checked before execution when annotations are present.
10. Every module environment is allocated before import linking.
11. Import aliases are live, read-only links to exported bindings.
12. Modules initialize once in deterministic dependency-first, source-import order.
13. Module environments and source identities remain valid for imported functions and diagnostics for the duration of one `run_path` call.

## 5. Front-end and checker boundaries

- Lexer token kinds and parser grammar are unchanged in V0.4; bytecode is downstream of the AST.
- Parser nesting is bounded; function parameter count is capped at 255.
- The checker is invoked when the source/module contains annotations.
- The checker handles binding mutability, typed function signatures, expression compatibility, return-path checks, and typed provenance.
- The VM must not replace static diagnostics with later runtime failures, nor skip the checker for annotated programs.

## 6. Existing regression corpus and gaps to close

The existing regression suite is embedded in `compiler/crates/hk/src/lib.rs` and in the repository's examples/integration tests. It already includes explicit regressions for:
- gradual typing and static incompatibility diagnostics;
- runtime variable/parameter/return contracts and contract aliasing;
- preserving previous binding values after failed contract writes;
- typed and dynamic diagnostic classification;
- short-circuit behavior;
- arrays, array mutation, identity/cycle safety;
- functions, recursion, scopes, shadowing, and return behavior;
- module resolution, exports/imports, aliases, module isolation, dependency order, and `std:math`;
- documented example execution.

This inventory is not sufficient by itself to prove bytecode conformance. M0/M6 must add a differential harness that can execute the same parsed program on both engines and compare output, error code, source identity/position, and relevant side-effect boundaries. Any semantic case without a targeted regression must receive one before VM rollout.

## 7. Required V0.4 differential matrix

| Area | Required differential cases |
|---|---|
| Literals/operators | Every literal/operator; precedence; mixed invalid operands; finite overflow |
| Evaluation order | Nested calls, callee/argument side effects, array receiver/index/value side effects |
| Short-circuit | Both branches skipped/evaluated; nested logical chains |
| Bindings | Duplicate declarations, const assignment, shadowing, outer assignment, block exit |
| Functions | Recursion, zero/many args, wrong arity, non-callable, fallthrough, nested returns |
| Contracts | Variable init/write, parameter, return, aliases, dynamic values, failed write atomicity |
| Arrays | Empty/nested/heterogeneous arrays, identity, mutation, invalid index, cycle rejection, length |
| Control flow | If/else, while, all for-clause omissions, return from nested control flow |
| Modules | Named imports/aliases, missing exports, duplicate names, cycles, module isolation, live alias, order, one-time initialization |
| Standard library | Every `std:math` function, arity, invalid types, numeric edge cases |
| Diagnostics | Code, source identity, line/column, typed/dynamic provenance |
| Limits | Parser nesting, call depth, module depth/count/source limits, VM stack/frame/code limits |

## 8. M0 exit criteria

M0 is complete only when:
- [x] Every current AST variant is enumerated.
- [x] Runtime value, binding, evaluation, operation, and module semantics are documented.
- [ ] Existing tests are mapped to named semantic cases (not merely inferred from file presence).
- [ ] Missing differential regressions are added before VM rollout.
- [ ] VM-specific resource ceilings are selected from measured stress tests.
- [ ] The exact baseline commit and CI status are recorded with reproducible links.

**Current conclusion:** the language surface is now enumerated. M0 remains open; no claim of implementation readiness or V0.4 completion is made until the remaining checkboxes are satisfied.
