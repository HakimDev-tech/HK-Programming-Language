# HK 0.4.0 — Compiler, VM, and end-to-end benchmarks

**Environment:** same 2-vCPU Linux sandbox and Rust toolchain for both versions  
**Iterations:** 25 VM executions per compile; 25 end-to-end runs per workload; median of three invocations  
**V0.3 reference:** a3ffadb923fef7032fdb6fbc5983b4cf5957886a  
**V0.4 candidate:** current bytecode-VM branch  
**Benchmark-only feature:** bench-internals (not enabled by default)

## Compiler and VM measured separately

The execution benchmark compiles, statically checks, and verifies each source program once, then executes the same verified bytecode 25 times without recompiling or cloning the program. This isolates compile/verification latency from VM execution latency.

| Workload | Compile + verify | VM average per execution |
|---|---:|---:|
| Arithmetic and loops | 114.0 µs | 1.893 ms |
| Recursive calls | 91.1 µs | 674 µs |
| Function calls | 65.3 µs | 2.569 ms |
| Nested closure calls | 109.7 µs | 1.515 ms |
| Array construction/index/mutation | 109.4 µs | 3.415 ms |

These values are medians across three release benchmark invocations. VM average is the elapsed time for 25 executions divided by 25. The compiler column includes parsing, static checks, AST compilation, and mandatory bytecode verification.

## Same-harness end-to-end comparison

The separate execution_e2e benchmark uses the public run and run_path APIs on both the V0.3 reference worktree and V0.4. Each result is the median average of three release invocations, 25 runs per invocation.

| Workload | V0.3 tree-walker | V0.4 VM | V0.4 change |
|---|---:|---:|---:|
| Arithmetic and loops | 1.581 ms | 1.919 ms | +21.3% |
| Recursive calls | 587 µs | 720 µs | +22.6% |
| Function calls | 1.980 ms | 2.514 ms | +27.0% |
| Nested closure calls | 1.242 ms | 1.534 ms | +23.6% |
| Array construction/index/mutation | 2.977 ms | 3.136 ms | +5.3% |
| Linked modules, 100 imported calls | 336 µs | 434 µs | +29.4% |

The end-to-end path includes parsing and static checking in both versions; V0.4 additionally compiles and verifies bytecode before VM execution. The module workload also includes dependency discovery, source loading, graph validation, linking, and execution.

## Interpretation

- V0.4 is an architectural bytecode/VM release, not a speed release. The measured end-to-end regressions are documented and accepted as a V0.4 trade-off; no speedup claim is made.
- The isolated VM numbers show the execution cost once bytecode is compiled and verified. The end-to-end comparison shows the cost of the current public one-shot API, which recompiles and verifies on each run.
- Profiling and reducing repeated compilation/verification costs are appropriate follow-up work; do not weaken mandatory verification to improve benchmark numbers.
- Measurements are host- and workload-specific, not production hardware guarantees.

## Reproduction

Run compiler/VM-separated measurements:

    cargo bench -p hk --features bench-internals --bench execution

Run public end-to-end measurements:

    cargo bench -p hk --features bench-internals --bench execution_e2e

For V0.3, check out the reference commit, copy execution_e2e.rs into its benches directory, add the bench-internals feature and same bench target to Cargo.toml, then run the same command. The benchmark sources are in compiler/crates/hk/benches.
