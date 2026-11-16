# HK 0.4.x — Compiler ABI gaps and lexical-capture contract

Status: the bytecode representation and verifier now contain the lexical-capture
foundation on feat/hk-v0.4.0-bytecode-vm. The AST compiler and VM do not yet
exist. This document records the constraints the compiler and VM must implement;
it is not a claim that closure execution is already available.

## 1. V0.3 lexical-environment contract

The V0.3 interpreter stores a function's defining lexical environment as a
Weak<RefCell<Env>> in Fun::env. Nested named functions can resolve and mutate
ancestor bindings while the defining environment is alive. The module loader
separately keeps defining module environments alive for imported functions
during one run_path execution.

The VM must preserve those ownership and lifetime boundaries. It must not turn
the V0.3 weak lexical-environment relationship into an indefinitely strong
escaping-closure guarantee. A function whose defining environment has expired
must retain the reference runtime's controlled
Function environment is no longer available failure.

## 2. Bytecode ABI now present in this branch

FunctionProto now contains:
- enclosing_function: Option<u32> — lexical parent prototype, absent for roots;
- upvalues: Vec<UpvalueDescriptor> — ordered capture table;
- each descriptor records UpvalueSource::EnclosingLocal(slot) or
  UpvalueSource::EnclosingUpvalue(slot), mutability, and optional type contract.

The opcode set now contains:
- GetUpvalue(u16) — push a captured binding's current value;
- SetUpvalue(u16) — assign the top stack value while preserving the expression
  result, with constant/import write enforcement left to runtime semantics.

The compiler ABI version is 2. Programs using upvalues must declare feature bit
FEATURE_LEXICAL_UPVALUES. The verifier rejects unsupported ABI/features.

## 3. Verifier guarantees now present

- A capture-bearing function must identify a valid earlier lexical parent.
- Local captures and forwarded captures must resolve to existing parent slots.
- Capture mutability and type-contract metadata must exactly match the source.
- Duplicate capture descriptors are rejected.
- A nested prototype can only be instantiated by its declared lexical parent.
- A nested function cannot capture a parent local before definite initialization.
- Upvalue opcode indices and stack effects are verified.
- Assignment to immutable captures remains a runtime decision, matching the
  existing verifier policy for local/module/import writes and preserving
  unreachable-code behavior.
- The malformed-bytecode fuzzer now emits upvalue opcodes and lexical-capture
  metadata, including invalid cases.

## 4. Remaining compiler/VM obligations

The compiler and VM must implement all of the following; the verifier metadata
alone is insufficient:

- Capture shared bindings, not copied values. Repeated references to the same
  binding must preserve identity, mutation, and contracts.
- Forward transitive captures through intermediate functions.
- Preserve self-recursive and mutually recursive named-function declarations.
- Preserve lexical shadowing, scope exit, and weak environment expiration.
- Ensure captured const/import assignment fails at the same runtime boundary
  and with the same diagnostic as V0.3.
- Preserve typed variable, parameter, and return contracts through captures.
- Define native prelude resolution for print and all std:math functions,
  including lexical shadowing and existing arity/type/domain diagnostics.
- Compile every AST node or return an explicit diagnostic at a documented
  unsupported boundary. Imports/exports must never be silently discarded.
- Verify every compiled program before execution.

## 5. Acceptance gate

M3 is not complete until compiler tests cover every AST variant, nested scopes,
shadowing, recursion, captured mutable/immutable bindings, typed captures,
short-circuiting, arrays/indexing/properties, source locations, and malformed
program rejection.

M4 is not complete until VM tests execute these programs and match the
reference interpreter's values, output, error codes/messages/positions, and
environment-lifetime behavior. M5/M6 module integration, differential testing,
fuzzing, and resource stress gates remain mandatory before release.
