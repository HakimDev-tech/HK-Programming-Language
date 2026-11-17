# HK 0.4.0 — VM benchmark baseline

**Implementation:** feat/hk-v0.4.0-bytecode-vm  
**Environment:** 2-vCPU Linux sandbox  
**Command:** cargo bench -p hk --bench execution

The harness measures the public, end-to-end execution paths. Each iteration includes parsing, static checks where applicable, bytecode compilation, mandatory verification, and VM execution. The linked-module case additionally includes module discovery, source loading, graph validation, import linking, compilation/merging, and execution.

## Baseline

Median of three consecutive release benchmark invocations, 25 iterations per workload:

| Workload | Median average per iteration |
|---|---:|
| Arithmetic and 1,000 loop iterations | 2.080 ms |
| 500 nested closure calls | 1.790 ms |
| 1,000 array mutations | 3.546 ms |
| Linked modules, 100 imported calls | 434 µs |

These are initial absolute measurements, not a comparison against the V0.3 tree-walker and not a production hardware claim. The small workloads are sensitive to host contention and filesystem/cache state. Re-run on the same host and toolchain when comparing later commits. Do not infer a V0.4 speedup from these values alone.

## Reproduction

    cargo bench -p hk --bench execution

The harness uses no third-party benchmarking dependency. It reports total elapsed time, iteration count, average iteration time, and output size. Workloads are fixed in compiler/crates/hk/benches/execution.rs.
