use std::{env, fs, process::ExitCode};
fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.as_slice() {
        [cmd] if cmd == "version" || cmd == "--version" || cmd == "-V" => {
            println!("HK {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [cmd] if cmd == "help" || cmd == "--help" || cmd == "-h" => {
            help();
            ExitCode::SUCCESS
        }
        [cmd, path] if cmd == "run" => run_file(path),
        _ => {
            help();
            ExitCode::from(64)
        }
    }
}
fn run_file(path: &str) -> ExitCode {
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error[E4001]: cannot read {path}: {e}");
            return ExitCode::from(74);
        }
    };
    match hk::run(path, &source) {
        Ok(lines) => {
            for line in lines {
                println!("{line}");
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            let code = if e.code == "E3000" { 1 } else { 2 };
            eprintln!("{e}");
            ExitCode::from(code)
        }
    }
}
fn help() {
    println!("HK — programming language\n\nUsage:\n  hk run <file.hk>\n  hk version\n  hk help");
}
