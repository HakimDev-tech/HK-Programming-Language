# HK 0.3.0 — Implementation and Release Plan

**Status:** HK 0.3.0 is merged into `main`; all technical gates passed. Git tag and GitHub Release publication remain pending.
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
- [x] Full CI, fuzz, Clippy, rustdoc, and cross-platform validation on the final release candidate.
- [x] Complete acceptance matrix, robustness audit, documentation audit, version bump, and merge.
- [ ] Publish the `v0.3.0` Git tag and GitHub Release.

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

- [x] All normative acceptance criteria in HK-MODULES-SPEC.md pass by implementation audit and regression coverage.
- [x] Full workspace test suite passes without deleting or weakening legacy tests.
- [x] `cargo fmt --all -- --check` passes.
- [x] `cargo clippy --workspace --all-targets -- -D warnings` passes.
- [x] `cargo doc --workspace --no-deps` passes.
- [x] Linux, Windows, and macOS CI pass on the exact release-candidate head.
- [x] Dedicated parser/interpreter and module-resolver fuzz campaigns pass on the exact release-candidate head.
- [x] Documented core, gradual-typing, module, and CLI examples are covered by automated execution tests.
- [x] Public API, diagnostics, compatibility, and known limitations are audited.
- [x] Crate version, README, changelog, architecture, language spec, and roadmap agree on 0.3.0.
- [x] No unresolved PR review threads or blocking review comments exist.
- [x] PR #14 merged into `main` by squash commit `644cee85f839764b146c024ab04972861bd677fe`.
- [ ] Publish the `v0.3.0` Git tag and GitHub Release.

## 4. Release-candidate evidence\n\n- Release-candidate head: `fe8dcc95a9a4894e9d3bb1bcde8bbb02fe6cb8c4`.\n- Cross-platform CI run: `43760570742` (Linux, Windows, macOS all successful).\n- Fuzz run: `43760570755` (parser/interpreter and module-resolver jobs successful).\n- CI commands: `cargo fmt --all -- --check`, `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, and `cargo doc --workspace --no-deps`.\n- PR: [#14](https://github.com/HakimDev-tech/HK-Programming-Language/pull/14).\n\n## 5. Deferred beyond 0.3.0

Package management, third-party dependencies, lockfiles, namespace imports, cyclic modules, re-exports, default exports, dynamic imports, full static type-interface propagation, additional standard-library modules, bytecode/VM, native compilation, WASM, FFI, concurrency, closures, generics, classes, and traits.
