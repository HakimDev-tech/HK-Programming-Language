# HK 0.4.0 — End-to-end benchmark comparison

**Environment:** same 2-vCPU Linux sandbox and Rust toolchain for both revisions  
**Iterations:** 25 per workload per invocation; median of three release invocations  
**V0.3 reference:** a3ffadb923fef7032fdb6fbc5983b4cf5957886a  
**V0.4 candidate:** a85ac1408eed7967d6abdc8f076d5e43f2a2e9f8  
**Command:** cargo bench --bench execution

The identical benchmark harness was copied into a detached V0.3 worktree and run against both public APIs. Each iteration includes parsing, static checks where applicable, and execution. V0.4 additionally includes AST-to-bytecode compilation and mandatory bytecode verification. The linked-module workload also includes module discovery, source loading, graph validation, import linking, and execution.

## Results

| Workload | V0.3 tree-walker | V0.4 VM | V0.4 change |
|---|---:|---:|---:|
| Arithmetic and 1,000 loop iterations | 1.692 ms | 2.273 ms | +34.3% slower |
| 500 nested closure calls | 1.392 ms | 1.863 ms | +33.8% slower |
| 1,000 array mutations | 2.900 ms | 3.304 ms | +13.9% slower |
| Linked modules, 100 imported calls | 375 µs | 534 µs | +42.3% slower |

These are medians of three consecutive invocations, with 25 iterations per invocation. They show an end-to-end throughput/latency regression for the tested workloads; they do not prove that VM instruction execution alone is slower. V0.4's public path includes compilation and mandatory verification on each run, while V0.3 directly interprets the AST. The contribution of compilation, verification, dispatch, and runtime operations has not yet been isolated with a profiler.

## Interpretation and release decision

- Do not market V0.4 as a performance improvement. The current evidence shows the opposite for these end-to-end workloads.
- The benchmark comparison gate is complete, but the regression must be explicitly accepted for the V0.4 architectural release or addressed through optimization before release approval.
- If optimizing, profile compile/verify and VM execution separately before changing semantics or weakening mandatory verification.
- Results are host- and workload-specific. Re-run on the same host/toolchain after any performance change.

## Reproduction

Run on V0.4:

    cargo bench -p hk --bench execution

For the V0.3 baseline, check out the V0.3 reference commit, copy compiler/crates/hk/benches/execution.rs into its benches directory, add the same [[bench]] target to compiler/crates/hk/Cargo.toml, and run cargo bench --manifest-path compiler/crates/hk/Cargo.toml --bench execution.

The harness uses no third-party benchmarking dependency. It reports total elapsed time, iteration count, average iteration time, and output size. Workloads are fixed in compiler/crates/hk/benches/execution.rs.
