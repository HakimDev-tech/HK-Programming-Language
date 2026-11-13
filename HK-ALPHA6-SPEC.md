# HK 0.2.0 — Alpha.6 Scope and Loop-Analysis Conformance Specification

**Status:** Normative design and acceptance specification. No Alpha.6 implementation is authorized by this document alone.
**Design baseline:** Alpha.5 commit `a4cb3d975a77defcb482dac944e6ccfbc5cc30f4`.
**Target:** Make static scope analysis and loop-clause analysis conservative and consistent with the existing tree-walking runtime, without changing the language syntax or runtime scope behavior.

## 1. Objective

Alpha.6 is a bounded correctness increment for the gradual type checker. It addresses discrepancies between the checker's synthetic scopes and the runtime's actual environment creation, and makes the static analysis order of `for` clauses match runtime evaluation order.

Alpha.6 adds no source syntax, source-level types, inference, conversions, runtime values, or backend. It must preserve the 0.1.x runtime behavior and all Alpha.1–Alpha.5 contracts.

## 2. Audit findings

The Alpha.5 implementation has the following concrete discrepancies:

1. `TypeChecker::check_statement` creates an extra scope around each `if` branch. The runtime does not create an environment for an `if` branch; only an explicit `S::Block` creates one.
2. The checker similarly creates an extra scope around a `while` body, while the runtime executes the body in the current environment unless the body itself is a block.
3. The checker checks a `for` increment before checking its body. The runtime order is initializer, condition, body, increment.
4. The current checker therefore may resolve declarations differently from the runtime and may analyze a loop increment against a static environment that does not correspond to the runtime point at which the increment executes.

These are checker/runtime conformance defects, not requests to redesign runtime scoping.

## 3. Normative scope model

### 3.1 Scope creation

The checker must model the existing runtime rules:

- An explicit block statement (`S::Block`) creates a lexical child scope.
- An `if` statement does not itself create a scope.
- A `while` statement does not itself create a scope.
- A `for` statement creates one loop scope covering its initializer, condition, body, and increment; that scope ends after the loop.
- A branch or loop body that is itself an explicit block creates the block scope through the normal `S::Block` rule.
- No synthetic scope may be introduced solely because a statement is used as a control-flow body.

### 3.2 Conditional declarations

The runtime executes only the selected `if` branch and may execute a `while` body zero or more times. Declarations in these bodies are therefore not guaranteed to have executed after the control-flow statement.

To remain conservative without adding definite-assignment analysis:

- Analyze each `if` branch from the same incoming static environment.
- Do not promote declarations introduced only inside a conditional branch into definite bindings after the `if`.
- Analyze a `while` body against the incoming environment, but do not promote declarations introduced only in the loop body into definite bindings after the loop.
- Within a branch/body, declarations are visible according to the current lexical scope and explicit block structure.
- Names not definitely available after a control-flow statement retain the existing dynamic/unresolved static treatment (`any`). Runtime lookup remains authoritative and may still report an undefined name.
- Assignments to bindings that existed before the branch/body must be checked against their existing contracts.
- Do not add flow-sensitive narrowing, definite-assignment diagnostics, or a new duplicate-declaration diagnostic in Alpha.6.

This is a conservative static approximation. It does not change the runtime fact that a declaration executed in an unbraced branch/body is installed in the current runtime environment.

### 3.3 Branch isolation

The checker must not let declarations from the `then` branch contaminate analysis of the `else` branch, or vice versa. Each branch is checked from an equivalent copy/snapshot of the incoming static environment. Existing outer bindings remain visible and retain their contracts.

For declarations that collide with an existing binding in the same runtime scope, the checker must not invent a new shadowing scope. Existing runtime duplicate-declaration behavior remains authoritative; Alpha.6 must not silently treat such a declaration as a valid nested shadow.

## 4. Normative `for` analysis order

The checker must analyze the clauses in runtime order:

1. Enter the loop scope.
2. Check the initializer, if present.
3. Check the condition, if present.
4. Check the body.
5. Check the increment, if present, using the static environment resulting from the body analysis.
6. Exit the loop scope.

Additional rules:

- An omitted condition is an unconditional loop and has no condition expression to check.
- An explicit block body creates and exits its own block scope before the increment is analyzed.
- An unbraced body is analyzed in the loop scope; declarations introduced directly by that body may therefore be visible to the increment, consistent with runtime execution order.
- The loop scope and all declarations within it are discarded after the loop.
- This ordering is a static source-analysis order. It must not execute source expressions or alter runtime evaluation.

## 5. Static environment architecture

The implementation may use cloned scope stacks, snapshots, or an equivalent internal abstraction. The chosen design must satisfy all of the following:

- Branch analysis starts from a stable incoming environment.
- Mutations to checker bookkeeping in one branch cannot leak into another branch.
- Conditional declarations are not treated as definitely available after the branch.
- Existing outer binding types/contracts remain available inside branches.
- A `for` body is analyzed before its increment.
- Explicit block scope entry/exit remains centralized and balanced, including when an error occurs.
- No public API or AST change is introduced unless the existing representation makes these rules impossible to implement.

Do not introduce a general control-flow graph, SSA, flow-sensitive narrowing, or global inference in this alpha.

## 6. Compatibility invariants

- Runtime environment creation and execution order remain unchanged.
- Unannotated 0.1.x behavior remains unchanged.
- Existing typed binding and function contracts remain enforced by the runtime.
- Existing declaration-order and function-resolution rules remain unchanged.
- Function declarations and aliases retain Alpha.5 behavior.
- Array binding contracts, heterogeneous elements, aliasing, and mutation semantics remain unchanged.
- Short-circuit evaluation remains lazy at runtime.
- Static checking still completes before runtime execution.
- A static error still prevents all runtime side effects.
- Existing diagnostic codes remain stable. Alpha.6 must not reclassify ordinary runtime errors or typed contract failures.

## 7. Required regression cases

### 7.1 If branches

- An outer typed binding remains visible and checked inside an unbraced `if` branch.
- An explicit block branch creates exactly one additional lexical scope.
- Declarations in the `then` branch do not leak into static analysis of the `else` branch.
- Declarations introduced only in a branch are not considered definitely available after the `if`.
- A branch declaration must not be treated as a valid nested shadow when runtime would encounter a same-scope duplicate.
- A typed assignment to an outer binding inside either branch is statically checked against the outer contract.

### 7.2 While loops

- Outer bindings remain visible and checked in an unbraced `while` body.
- An explicit block body has its own scope.
- Body-only declarations are not promoted to definite bindings after the loop.
- A body declaration does not become visible in a sibling or enclosing scope through checker-state leakage.
- Existing runtime behavior for zero iterations and repeated iterations remains unchanged.

### 7.3 For loops

- Initializer declarations are visible in condition, body, and increment.
- The body is analyzed before the increment.
- An unbraced body declaration is visible to the increment when the runtime would execute the body before reaching the increment.
- A declaration inside an explicit block body is not visible to the increment.
- Loop-scope declarations are not visible after the loop.
- Omitted initializer, condition, and increment combinations remain valid according to the existing grammar.
- A missing condition remains an unconditional loop.

### 7.4 Error and state safety

- Checker scope stacks are restored after success and after every error path.
- A failed branch/body check does not contaminate later checker state.
- No source expression is evaluated during static analysis.
- Existing 0.1.x, Alpha.1–Alpha.5 tests remain intact and passing.

## 8. Acceptance gates

Alpha.6 is complete only when:

1. The scope model in Section 3 is implemented and covered by focused tests.
2. The `for` analysis order in Section 4 matches runtime order.
3. All required regression categories in Section 7 are covered.
4. Existing gradual-typing matrix tests pass unchanged.
5. Existing V0.1.x regression tests pass unchanged.
6. Static errors continue to prevent runtime side effects.
7. No runtime semantics, source syntax, type vocabulary, or public API changes are introduced.
8. Formatting, workspace tests, Clippy with warnings denied, and rustdoc with warnings denied pass.
9. Linux, Windows, and macOS CI pass on the exact final commit.
10. The dedicated parser/interpreter fuzz workflow passes on the exact final commit.
11. Documentation accurately describes Alpha.6 as a checker-conformance increment, not a new language feature or stable release.
12. The implementation PR reports exact commit SHAs and validation runs and remains unmerged until explicitly authorized.

## 9. Explicit non-goals

No definite-assignment analysis, control-flow graph, SSA, flow-sensitive narrowing, type inference, new diagnostics taxonomy, new syntax, new types, generics, unions, optional types, typed array elements, modules, closures, bytecode, VM, native compilation, WASM, FFI, or standard-library expansion.

## 10. Definition of done

The Alpha.6 implementation must be developed on a branch based on the validated Alpha.5 head `a4cb3d975a77defcb482dac944e6ccfbc5cc30f4`. It must have a dedicated acceptance matrix and exact-head CI/fuzz validation. Alpha.5 and earlier PRs remain unmerged; `main` and the Alpha.4 baseline remain untouched. Alpha.6 is not a release candidate.
