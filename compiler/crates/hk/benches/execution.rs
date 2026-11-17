use hk::{run, run_path};
use std::fs;
use std::hint::black_box;
use std::time::{Duration, Instant};

const ITERATIONS: usize = 25;

fn measure(name: &str, source: &str) {
    let started = Instant::now();
    let mut output_bytes = 0usize;
    for _ in 0..ITERATIONS {
        let output = run("benchmark.hk", black_box(source)).expect("benchmark program must run");
        output_bytes += output.iter().map(String::len).sum::<usize>();
        black_box(output);
    }
    report(name, started.elapsed(), output_bytes);
}

fn report(name: &str, elapsed: Duration, output_bytes: usize) {
    println!(
        "{name}: total={elapsed:?}, iterations={ITERATIONS}, average={:?}, output_bytes={output_bytes}",
        elapsed / ITERATIONS as u32
    );
}

fn measure_modules() {
    let root = std::env::temp_dir().join(format!("hk-v040-bench-{}", std::process::id()));
    fs::create_dir_all(&root).expect("create benchmark directory");
    fs::write(
        root.join("library.hk"),
        "export let count = 0; export fn next() { count = count + 1; return count; }",
    )
    .expect("write module");
    fs::write(
        root.join("main.hk"),
        "import { next } from \"./library.hk\"; let total = 0; for (let i = 0; i < 100; i = i + 1) { total = total + next(); } print(total);",
    )
    .expect("write entry module");

    let started = Instant::now();
    let mut output_bytes = 0usize;
    for _ in 0..ITERATIONS {
        let output = run_path(root.join("main.hk")).expect("module benchmark must run");
        output_bytes += output.iter().map(String::len).sum::<usize>();
        black_box(output);
    }
    report("linked-modules", started.elapsed(), output_bytes);
    fs::remove_dir_all(root).expect("remove benchmark directory");
}

fn main() {
    measure(
        "arithmetic-and-loops",
        "let total = 0; for (let i = 0; i < 1000; i = i + 1) { total = total + i; } print(total);",
    );
    measure(
        "closure-calls",
        "fn outer(seed) { let value = seed; fn next() { value = value + 1; return value; } let total = 0; for (let i = 0; i < 500; i = i + 1) { total = total + next(); } print(total); } outer(0);",
    );
    measure(
        "array-mutation",
        "let values = [0, 0, 0, 0]; for (let i = 0; i < 1000; i = i + 1) { values[i % 4] = values[i % 4] + 1; } print(values[0] + values[1] + values[2] + values[3]);",
    );
    measure_modules();
}
