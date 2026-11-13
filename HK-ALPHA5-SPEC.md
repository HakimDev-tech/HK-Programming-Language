# HK 0.2.0 — Alpha.5 Boundary Conformance Specification

**Status:** Proposed normative specification for Alpha.5; implementation has not started.
**Baseline:** Alpha.4 stabilization head `8445732178148ae7295509d5630622ec2547dbf5`.
**Target:** Improve conformance, diagnostics, and regression coverage at static/dynamic boundaries without expanding the language surface.

## 1. Objective

Alpha.5 is a correctness and observability increment for the gradual-typing implementation. It does not add syntax, types, inference, or runtime features.

The work must make the existing contract between static checking and runtime enforcement explicit and testable. The observable behavior of programs without annotations must remain compatible with the V0.1.x baseline, except for defects explicitly documented and approved before implementation.

## 2. Normative invariants

1. Parsing and static checking complete before program execution begins.
2. A static type error prevents all program side effects.
3. A known incompatible type is rejected statically; a value whose static type is `any` is checked at the relevant runtime boundary.
4. Runtime contract validation occurs before the operation it protects. A failed assignment must not replace the previous binding value; a failed function-parameter check must not enter the function body; a failed return check must not complete the call successfully.
5. A contract belongs to the annotated binding or function boundary, not to an array object, its elements, or every alias of the referenced value.
6. Function aliases preserve the function's parameter and return contracts.
7. Short-circuit operators evaluate the right operand only when required by their existing semantics. A skipped operand must not trigger a runtime guard or side effect.
8. Ordinary dynamic runtime failures remain `E3000`. Violations of typed contracts or runtime type guards remain `E3008`.
9. Runtime validation must not evaluate a source expression more than once.
10. No runtime failure covered by this specification may cause a Rust panic.

## 3. Diagnostic provenance

For every runtime contract/guard failure, the diagnostic must point to the most specific source expression that caused the failed boundary check, using the source positions already carried by the AST wherever possible.

| Boundary | Required diagnostic location |
|---|---|
| Typed variable initialization | Typed declaration |
| Assignment to a typed binding | Assignment expression |
| Direct or indirect function argument contract | The offending argument expression |
| Explicit function return contract | Returned expression; use the return statement location only when no expression exists |
| Implicit function return contract | Function declaration or call boundary, consistently documented by the implementation |
| Dynamic condition guard | Condition expression |
| Unary/binary operand guard | Offending operand expression |
| Array receiver guard | Receiver expression |
| Array index guard | Index expression |

The diagnostic code is `E3008` for contract/guard failures and `E3000` for ordinary runtime failures. Alpha.5 must not reclassify ordinary dynamic operator overload failures, non-callable calls, undefined names, bounds errors, or other legacy runtime failures as contract violations merely because the program contains annotations.

Diagnostic messages must identify the expected type and actual runtime type where that information is meaningful. Messages must remain deterministic and must not expose Rust implementation details.

## 4. Boundary conformance matrix

The implementation must add positive and negative tests for each applicable boundary below.

### 4.1 Binding boundaries

- Typed initialization from a statically known compatible value succeeds.
- Typed initialization from `any` succeeds only when the runtime value satisfies the declared type.
- Typed initialization from `any` with an incompatible runtime value fails with `E3008`.
- Assignment to a typed binding validates before mutation.
- After a failed assignment, the previous value remains readable and unchanged.
- Shadowed bindings maintain independent contracts.
- Unannotated bindings preserve dynamic assignment behavior.

### 4.2 Function boundaries

- Direct calls to known functions retain static arity and argument checks.
- Calls through aliases retain runtime parameter and return contracts.
- Each parameter is validated before the function body executes.
- A failed parameter check prevents all body side effects.
- Explicit return values are checked against declared return contracts.
- Implicit fallthrough behavior remains as specified in `HK-TYPING-SPEC.md`: permitted for `null` and `any`; rejected for concrete non-null return types when reachable.
- A failed return check does not produce a successful call result.
- Dynamic non-callable values and ordinary arity/runtime errors retain legacy classification unless the failure is specifically a typed contract violation.

### 4.3 Operator and control-flow guards

Test dynamic values at every existing guard site:

- unary `-` requires `number`;
- unary `!` requires `bool`;
- `-`, `*`, `/`, and `%` require numeric operands;
- order comparisons require numeric operands;
- `&&` and `||` require boolean operands and preserve short-circuit behavior;
- `if`, `while`, and conditional `for` require boolean conditions;
- array indexing requires an array receiver and numeric index;
- `.length` requires an array receiver.

For binary operations, diagnostics must identify the offending operand, not merely the enclosing statement. The `+` operator retains its two documented overloads (number + number and string + string); an unsupported dynamic combination is an ordinary runtime error (`E3000`), not a contract failure (`E3008`).

### 4.4 Array semantics

- Array contracts apply to the binding only.
- Elements remain dynamically typed.
- Aliases do not inherit a binding's contract.
- Mutating an array through an alias remains observable through all references.
- Existing cycle prevention, bounds checks, and identity equality remain unchanged.
- Runtime guards must not accidentally impose element-type checks.

### 4.5 Scope and function resolution

Add explicit tests for:

- function aliases in the same scope and nested scopes;
- a variable shadowing a function name;
- a nested function declaration shadowing an outer binding;
- independent contracts for shadowed typed bindings;
- direct recursion;
- rejection/legacy behavior for forward references and mutual recursion;
- calls through dynamic aliases;
- arity and parameter-contract failures through aliases.

The checker and runtime must agree on lexical name resolution. If a mismatch is found, correct it without broadening the language's declaration-order rules.

## 5. Evaluation and side-effect guarantees

Tests must use observable output or state to prove ordering, not only error codes.

- Static errors: no program output or other runtime side effects occur.
- Failed typed initialization: the initializer is evaluated once; no binding is installed.
- Failed typed assignment: the right-hand expression is evaluated once; the old binding value remains intact.
- Failed parameter validation: argument expressions are evaluated once in the established order; the function body is not entered.
- Failed return validation: the return expression is evaluated once; the caller receives an error, not a value.
- Short-circuit: the skipped branch is neither evaluated nor guarded.

Where the public API currently returns only an error rather than partial output, tests may verify the same guarantees with a test-only observable native/test harness or a carefully constructed subsequent state observation. Do not add public language features solely for testing.

## 6. Compatibility requirements

- Preserve all existing V0.1.x tests and their behavior.
- Preserve all Alpha.1–Alpha.4 acceptance tests.
- Do not change the semantics of unannotated programs as a side effect of typed-mode diagnostics.
- Do not introduce implicit conversions.
- Do not change the current array model, declaration-order rules, function model, or type lattice.
- Any discovered incompatibility must be isolated, documented, and covered by a regression test.

## 7. Implementation constraints

Expected primary files:

- `compiler/crates/hk/src/runtime.rs`: boundary validation and source provenance.
- `compiler/crates/hk/src/type_checker.rs`: static/runtime resolution consistency audit.
- `compiler/crates/hk/tests/gradual_typing_matrix.rs`: expanded conformance matrix.
- focused unit tests in the existing crate test module where appropriate.
- `fuzz/corpus/run/`: minimized valid and invalid seeds for parser/interpreter boundary cases.

Changes to `ast.rs`, `parser.rs`, or `diagnostics.rs` are permitted only when current source-position data cannot represent the required diagnostic location. Such changes must remain minimal and preserve existing parser diagnostics.

No dependency, new module, new syntax, new type, or backend change is authorized by this Alpha.5 specification.

## 8. Required acceptance gates

Alpha.5 is complete only when all of the following are true:

1. All V0.1.x regression tests pass unchanged.
2. All Alpha.1–Alpha.4 tests pass.
3. Every boundary category in Section 4 has positive and negative coverage where applicable.
4. Diagnostic codes and source locations match Section 3.
5. Atomicity and evaluation-order guarantees in Section 5 are tested.
6. Static errors are proven to prevent runtime execution.
7. Function aliases and lexical shadowing are consistent between checker and runtime.
8. Array aliasing and binding-only contracts remain unchanged.
9. The fuzz corpus includes typed/dynamic boundary cases and completes the configured CI fuzz session without a crash.
10. Formatting, workspace tests, Clippy with warnings denied, and rustdoc with warnings denied pass.
11. Linux, Windows, and macOS CI pass on the exact final commit.
12. The normative typing specification, implementation plan, changelog, and README accurately describe the implemented behavior.

Passing a short fuzz session is evidence of robustness, not proof of absence of defects. Alpha.5 is not a stable release and must not be described as production-ready.

## 9. Explicit non-goals

No generics, union types, optional types, typed array elements, flow-sensitive narrowing, global inference, function types, anonymous functions, closures, modules, bytecode, VM, native compilation, WASM, FFI, package manager, or standard-library expansion.

## 10. Definition of done

The Alpha.5 implementation PR must link to this specification, enumerate each acceptance gate, report exact tested commit SHAs and CI/fuzz run results, and remain unmerged until explicitly authorized. The Alpha.4 baseline branch and `main` must not be modified by Alpha.5 work.
