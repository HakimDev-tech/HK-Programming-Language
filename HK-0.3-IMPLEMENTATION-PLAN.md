# HK 0.3.0 — Implementation and Release Plan

**Status:** Design approved as the implementation baseline for the 0.3.0 branch. No implementation item is considered complete until code, regression tests, and exact-commit CI evidence exist.
**Target:** HK 0.3.0 — Modules and Standard Library
**Base:** `main` at `9d09ea94836e1aa71987b7f45e9e718907a29e8f` (HK 0.2.0 source integration)

The normative language contract is [HK-MODULES-SPEC.md](HK-MODULES-SPEC.md). This plan does not create intermediate Alpha releases.

## 1. Architectural work

1. Extend AST and parser with top-level import and export declarations.
2. Add a module graph loader that separates path resolution, source loading, parsing, export validation, dependency ordering, and execution.
3. Canonicalize filesystem paths and cache modules by canonical identity.
4. Introduce per-module runtime environments and explicit import bindings.
5. Keep module environments alive for imported function closures and preserve export binding identity.
6. Add a standard-library registry for the reserved `std:` namespace and implement `std:math`.
7. Extend static checking with module-local declarations and imported-name bindings typed as `any` in 0.3.0.
8. Preserve source identity through every loaded module so diagnostics point to the correct file.

The loader must validate the complete dependency graph, duplicate declarations/exports/imports, and missing exports before executing any module body. Cycles are errors; partial initialization is not supported.

## 2. Required test suites

### Parser and AST
- Valid named imports, aliases, exports, and mixed typed/untyped exported declarations.
- Malformed grammar, reserved `std:` paths, empty paths, duplicate names, misplaced imports, nested exports.
- Source-position correctness for all new diagnostics.

### Resolver and graph
- Nested relative imports, parent-directory imports, and resolution independent of current working directory.
- Canonical-path deduplication through `.` and `..`.
- Missing file, directory target, unsupported extension, invalid UTF-8, and path read errors.
- Diamond dependency initialized once.
- Deterministic dependency order.
- Direct and indirect cycles rejected before side effects.
- Repeated import uses the same module instance.

### Runtime and typing
- Private top-level binding isolation.
- Named imports and aliases.
- Imported binding immutability.
- Exported `let` and `const` behavior.
- Exported function closure resolves private state in its defining module.
- Typed parameter, return, and binding contracts survive import.
- Existing dynamic behavior and diagnostic codes remain unchanged for legacy programs.

### Standard library
- Correct results for all seven functions.
- Wrong arity, wrong argument types, negative square root, and non-finite results.
- Source location at the importing call site.
- No hidden I/O or mutation.

### Regression and robustness
- Full 0.1.x/V0.2 suite unchanged and passing.
- Parser and module-loader fuzzing.
- Path normalization and symlink/canonicalization cases where supported by the platform.
- No uncontrolled panic on malformed source or dependency graphs.

## 3. Release gates

- [ ] All normative acceptance criteria in HK-MODULES-SPEC.md pass.
- [ ] Full workspace test suite passes without deleting or weakening legacy tests.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` passes.
- [ ] Linux, Windows, and macOS CI pass on the exact proposed merge commit.
- [ ] Dedicated fuzz campaign passes on the exact release candidate code.
- [ ] Every documented example executes successfully.
- [ ] Public API, diagnostics, compatibility, and known limitations are audited.
- [ ] Crate version, README, changelog, architecture, language spec, and roadmap agree on 0.3.0.
- [ ] PR review has no unresolved blocking threads.
- [ ] Tag and GitHub Release are published only after all technical gates pass.

## 4. Deferred beyond 0.3.0

Package management, third-party dependencies, lockfiles, namespace imports, cyclic modules, re-exports, default exports, dynamic imports, full static type-interface propagation, additional standard-library modules, bytecode/VM, native compilation, WASM, FFI, concurrency, closures, generics, classes, and traits.
