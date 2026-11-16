# HK 0.4.x — Compiler ABI gaps that must be resolved before M3

Status: engineering addendum, derived from the existing AST, semantic inventory,
bytecode model, verifier, and reference interpreter. This document does not claim
that the compiler or VM is implemented.

## 1. Existing named functions require lexical capture

The V0.3 interpreter stores a function's defining lexical environment as a
`Weak<RefCell<Env>>` in `Fun::env`. Nested named functions can resolve and
mutate ancestor bindings while the defining environment is alive. The module
loader separately keeps defining module environments alive for imported
functions during one `run_path` execution.

The VM must preserve those exact ownership and lifetime boundaries. In
particular, it must not silently turn the V0.3 weak lexical-environment
relationship into an indefinitely strong escaping-closure guarantee. A function
whose defining environment has expired must retain the reference runtime's
controlled `Function environment is no longer available` failure.

The current bytecode model has `MakeFunction`, `GetLocal`, and `SetLocal`,
but no explicit capture layout or upvalue access. A child function's local-slot
indices cannot identify a slot in its parent frame. Treating captured variables
as globals or copying their values would break shadowing, mutation, recursion,
and binding identity.

This is not a request to add anonymous closures: it is a requirement to preserve
existing named-function semantics.

### Required ABI extension

Before compiling nested functions, add:

- `FunctionProto.enclosing_function: Option<u32>` (none for entry/top-level
  functions; otherwise the lexical parent prototype);
- an ordered `upvalues: Vec<UpvalueDescriptor>` table per function;
- `UpvalueDescriptor` entries that distinguish capture from an enclosing local
  slot versus forwarding an enclosing upvalue;
- `GetUpvalue(u16)` and `SetUpvalue(u16)` opcodes;
- closure construction semantics for `MakeFunction` that reference shared
  binding cells with ownership/lifetime matching V0.3, not copied values.

A binding cell must preserve mutability and any runtime type contract. Capturing
the same binding more than once must preserve identity. Recursive local named
functions must resolve their own predeclared cell. Transitive captures must be
forwarded through intermediate functions. The capture representation must not
keep a lexical environment alive longer than V0.3 does.

### Verifier obligations

The verifier must reject:

- an upvalue descriptor without a valid lexical parent;
- local captures outside the parent's local-slot table;
- forwarded captures outside the parent's upvalue table;
- upvalue reads/writes outside the child's descriptor table;
- writes to immutable captured bindings, unless the established V0.3 runtime
  contract intentionally defers that diagnostic to execution;
- inconsistent parent/child function relationships.

The verifier's abstract state must account for upvalue operations and their
stack effects. It must not infer lexical validity from source names.

## 2. Built-in bindings need a defined bytecode path

The current opcode set has `LoadPrint`, but the V0.3 runtime also exposes seven
`std:math` functions (`abs`, `ceil`, `floor`, `round`, `sqrt`, `min`,
`max`). The compiler/VM contract must define how these native values enter
execution and how local shadowing takes precedence. Do not silently compile a
math identifier as an ordinary unresolved local.

The implementation must either define a verified native-load opcode/table or
represent built-ins as explicit immutable prelude bindings with stable,
verifiable identities. The latter must preserve existing diagnostics and
shadowing behavior.

## 3. Compilation scope must be explicit

The AST includes imports and exports, while the current bytecode model already
contains module/import/export tables. M3 must state whether it compiles a single
module AST or a complete dependency graph. The complete graph linker/loader
integration belongs to M5, but M3 must not silently drop import/export nodes.

Every AST variant must have one of:
- a defined lowering;
- a deliberate, diagnostic rejection at a documented API boundary; or
- an explicit later milestone with tests proving it cannot be mistaken for
  successfully compiled code.

## 4. Required sequencing

1. Amend bytecode ABI and verifier for lexical captures and native bindings.
2. Add verifier-focused adversarial tests for malformed capture metadata.
3. Implement AST lowering with exhaustive matches and explicit source mapping.
4. Compile every accepted AST construct and verify the resulting program.
5. Implement VM frames, closure cells, contracts, and native dispatch.
6. Differentially compare VM and tree-walking reference behavior before routing
   any public execution API through the VM.

## 5. Acceptance gate

M3 is not complete until compiler tests cover every AST variant, nested scopes,
shadowing, recursion, captured mutable/immutable bindings, typed captures,
short-circuiting, arrays/indexing/properties, source locations, and malformed
program rejection. M4 is not complete until these programs execute in the VM and
match the reference interpreter's values, output, diagnostic boundaries, and
environment-lifetime behavior.
