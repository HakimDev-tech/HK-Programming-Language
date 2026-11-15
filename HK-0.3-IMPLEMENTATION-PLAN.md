# HK 0.3.0 — Implementation and Release Plan

**Status:** Implementation underway on the dedicated 0.3.0 branch. The checklist is updated only after implementation evidence exists.
**Target:** HK 0.3.0 — Modules and Standard Library
**Base:** `main` at `9d09ea94836e1aa71987b7f45e9e718907a29e8f` (HK 0.2.0 source integration)

The normative language contract is [HK-MODULES-SPEC.md](HK-MODULES-SPEC.md). No intermediate Alpha releases are planned.

## 1. Implementation status

- [x] AST and lexer support for import/export declarations.
- [x] Parser support for named imports, aliases, and top-level exports.
- [x] Filesystem module graph discovery and canonical path identity.
- [x] Dependency-first execution and at-most-once module initialization.
- [x] Import/export linking and pre-execution missing-export validation.
- [x] Isolated module root environments and read-only live aliases.
- [x] Preserve defining module environments and source diagnostics for imported functions.
- [x] Initial `std:math` implementation.
- [x] Public `run_path` API and CLI integration.
- [x] Initial module integration tests and runnable example.
- [x] CLI integration test for filesystem-backed module execution.
- [x] Stable diagnostics for module errors, imported-binding immutability, and std:math failures.
- [x] Resource limits for module depth, count, per-file source size, and aggregate source size.
- [x] Dedicated module specifier/resolver fuzz target.
- [ ] Full CI, fuzz, Clippy, rustdoc, and cross-platform validation on the final release candidate.
- [ ] Complete acceptance matrix, robustness audit, documentation audit, version bump, merge, tag, and release.

## 2. Required test suites

### Parser and AST
- Valid named imports, aliases, exports, and mixed typed/untyped exported declarations.
- Malformed grammar, reserved `std:` paths, empty paths, duplicate names, misplaced imports, nested exports.
- Source-position correctness for all new diagnostics.

### Resolver and graph
- Nested relative imports, parent-directory imports, and resolution independent of process working directory.
- Canonical-path deduplication through `.` and `..`.
- Missing file, directory target, unsupported extension, invalid UTF-8, and path read errors.
- Diamond dependency initialized once.
- Deterministic dependency order.
- Direct and indirect cycles rejected before side effects with import-edge diagnostics.
- Repeated import uses the same module instance.

### Runtime and typing
- Private top-level binding isolation.
- Named imports and aliases.
- Imported binding immutability and live exported mutable bindings.
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
- Parser and module-loader fuzzing, including malformed import specifiers and filesystem resolution.
- Path normalization and symlink/canonicalization cases where supported by the platform.
- No uncontrolled panic on malformed source or dependency graphs.

## 3. Release gates

- [ ] All normative acceptance criteria in HK-MODULES-SPEC.md pass.
- [ ] Full workspace test suite passes without deleting or weakening legacy tests.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps` passes.
- [ ] Linux, Windows, and macOS CI pass on the exact proposed merge commit.
- [ ] Dedicated parser and module-loader fuzz campaigns pass on the exact release candidate code.
- [ ] Every documented example executes successfully.
- [ ] Public API, diagnostics, compatibility, and known limitations are audited.
- [ ] Crate version, README, changelog, architecture, language spec, and roadmap agree on 0.3.0.
- [ ] PR review has no unresolved blocking threads.
- [ ] Tag and GitHub Release are published only after all technical gates pass.

## 4. Deferred beyond 0.3.0

Package management, third-party dependencies, lockfiles, namespace imports, cyclic modules, re-exports, default exports, dynamic imports, full static type-interface propagation, additional standard-library modules, bytecode/VM, native compilation, WASM, FFI, concurrency, closures, generics, classes, and traits.
