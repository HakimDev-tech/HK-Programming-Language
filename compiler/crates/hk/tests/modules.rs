use hk::run_path;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT_PROJECT: AtomicUsize = AtomicUsize::new(0);

fn project(files: &[(&str, &str)], entry: &str) -> (PathBuf, PathBuf) {
    let id = NEXT_PROJECT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!("hk-modules-{}-{id}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    for (name, source) in files {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, source).unwrap();
    }
    (root.clone(), root.join(entry))
}

fn run_project(
    files: &[(&str, &str)],
    entry: &str,
) -> std::result::Result<Vec<String>, hk::HkError> {
    let (root, path) = project(files, entry);
    let result = run_path(path);
    fs::remove_dir_all(root).unwrap();
    result
}

#[test]
fn imports_exports_aliases_and_live_bindings_work_with_gradual_contracts() {
    let output = run_project(
        &[
            (
                "counter.hk",
                r#"
                    print("counter initialized");
                    export let count: number = 1;
                    export fn add(value: number) -> number {
                        count = count + value;
                        return count;
                    }
                "#,
            ),
            (
                "main.hk",
                r#"
                    import { add as increment, count } from "./counter.hk";
                    print(increment(2));
                    print(count);
                "#,
            ),
        ],
        "main.hk",
    )
    .unwrap();

    assert_eq!(output, vec!["counter initialized", "3", "3"]);
}

#[test]
fn shared_dependencies_initialize_once_in_dependency_first_order() {
    let output = run_project(
        &[
            ("base.hk", "print(\"base\"); export const value = 7;"),
            (
                "left.hk",
                "import { value } from \"./base.hk\"; print(\"left\"); export const left = value;",
            ),
            (
                "right.hk",
                "import { value } from \"./base.hk\"; print(\"right\"); export const right = value;",
            ),
            (
                "main.hk",
                "import { left } from \"./left.hk\"; import { right } from \"./right.hk\"; print(left + right);",
            ),
        ],
        "main.hk",
    )
    .unwrap();

    assert_eq!(output, vec!["base", "left", "right", "14"]);
}

#[test]
fn standard_math_functions_enforce_results_and_contracts() {
    let output = run_project(
        &[(
            "main.hk",
            r#"
                import { abs, ceil, floor, round, sqrt, min, max } from "std:math";
                print(abs(-4));
                print(ceil(2.1));
                print(floor(2.9));
                print(round(-2.5));
                print(sqrt(81));
                print(min(3, 9));
                print(max(3, 9));
            "#,
        )],
        "main.hk",
    )
    .unwrap();

    assert_eq!(output, vec!["4", "3", "2", "-3", "9", "3", "9"]);
}

#[test]
fn import_aliases_are_read_only() {
    let error = run_project(
        &[
            ("values.hk", "export let value = 1;"),
            (
                "main.hk",
                "import { value } from \"./values.hk\"; value = 2;",
            ),
        ],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E4007");
    assert!(error.message.contains("imported binding"));
}

#[test]
fn cycles_are_rejected_before_any_module_body_executes() {
    let error = run_project(
        &[
            (
                "a.hk",
                "import { b } from \"./b.hk\"; print(\"a executed\"); export const a = 1;",
            ),
            (
                "b.hk",
                "import { a } from \"./a.hk\"; print(\"b executed\"); export const b = 2;",
            ),
            ("main.hk", "import { a } from \"./a.hk\"; print(a);"),
        ],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E4005");
    assert!(error.message.contains("Cyclic module dependency"));
}

#[test]
fn missing_exports_are_link_errors() {
    let error = run_project(
        &[
            ("values.hk", "export const present = 1;"),
            (
                "main.hk",
                "import { absent } from \"./values.hk\"; print(absent);",
            ),
        ],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E4003");
    assert!(error.message.contains("does not export"));
}

#[test]
fn typed_modules_use_the_same_imported_binding_diagnostic() {
    let error = run_project(
        &[
            ("values.hk", "export let value: number = 1;"),
            (
                "main.hk",
                "import { value } from \"./values.hk\"; let check: number = 2; value = check;",
            ),
        ],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E4007");
}

#[test]
fn private_module_bindings_do_not_leak() {
    let error = run_project(
        &[
            (
                "values.hk",
                "let private_value = 42; export const public_value = 1;",
            ),
            (
                "main.hk",
                "import { public_value } from \"./values.hk\"; print(private_value);",
            ),
        ],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E3000");
    assert!(error.message.contains("Undefined variable"));
}

#[test]
fn import_declarations_must_be_in_the_module_header() {
    let error = run_project(
        &[(
            "main.hk",
            "let value = 1; import { abs } from \"std:math\";",
        )],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E2000");
}

#[test]
fn imported_functions_keep_their_defining_module_environment_alive() {
    let output = run_project(
        &[
            (
                "values.hk",
                "let private_value = 40; export fn reveal() { return private_value; }",
            ),
            (
                "main.hk",
                "import { reveal } from \"./values.hk\"; print(reveal());",
            ),
        ],
        "main.hk",
    )
    .unwrap();

    assert_eq!(output, vec!["40"]);
}

#[test]
fn typed_export_contracts_are_enforced_across_module_boundaries() {
    let error = run_project(
        &[
            (
                "values.hk",
                r#"
                    export let amount: number = 1;
                    export fn update(value: any) {
                        amount = value;
                    }
                "#,
            ),
            (
                "main.hk",
                "import { update } from \"./values.hk\"; let input: string = \"invalid\"; update(input);",
            ),
        ],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E3008");
    assert!(error.message.contains("Runtime type contract violation"));
}

#[test]
fn standard_math_uses_stable_arity_type_and_domain_diagnostics() {
    let arity = run_project(
        &[("main.hk", "import { abs } from \"std:math\"; abs();")],
        "main.hk",
    )
    .unwrap_err();
    assert_eq!(arity.code, "E3011");

    let argument_type = run_project(
        &[(
            "main.hk",
            "import { sqrt } from \"std:math\"; sqrt(\"bad\");",
        )],
        "main.hk",
    )
    .unwrap_err();
    assert_eq!(argument_type.code, "E3012");

    let domain = run_project(
        &[("main.hk", "import { sqrt } from \"std:math\"; sqrt(-1);")],
        "main.hk",
    )
    .unwrap_err();
    assert_eq!(domain.code, "E3013");
}

#[test]
fn canonical_paths_deduplicate_the_same_dependency() {
    let output = run_project(
        &[
            ("sub/placeholder.hk", ""),
            ("base.hk", "print(\"base\"); export const value = 5;"),
            (
                "left.hk",
                "import { value } from \"./base.hk\"; print(value); export const left = value;",
            ),
            (
                "right.hk",
                "import { value } from \"./sub/../base.hk\"; print(value); export const right = value;",
            ),
            (
                "main.hk",
                "import { left } from \"./left.hk\"; import { right } from \"./right.hk\"; print(left + right);",
            ),
        ],
        "main.hk",
    )
    .unwrap();

    assert_eq!(output, vec!["base", "5", "5", "10"]);
}

#[test]
fn module_preflight_rejects_builtin_shadowing_and_duplicate_top_level_names() {
    let builtin = run_project(&[("main.hk", "export const print = 1;")], "main.hk").unwrap_err();
    assert_eq!(builtin.code, "E4001");

    let duplicate =
        run_project(&[("main.hk", "let value = 1; const value = 2;")], "main.hk").unwrap_err();
    assert_eq!(duplicate.code, "E4001");
}

#[test]
fn unsupported_and_missing_filesystem_specifiers_are_rejected() {
    let unsupported = run_project(
        &[("main.hk", "import { value } from \"./values.txt\";")],
        "main.hk",
    )
    .unwrap_err();
    assert_eq!(unsupported.code, "E4002");

    let missing = run_project(
        &[("main.hk", "import { value } from \"./missing.hk\";")],
        "main.hk",
    )
    .unwrap_err();
    assert_eq!(missing.code, "E4002");
}

#[test]
fn module_source_size_is_bounded() {
    let oversized = format!("//{}", "x".repeat(1_048_576));
    let error = run_project(&[("main.hk", &oversized)], "main.hk").unwrap_err();
    assert_eq!(error.code, "E4006");
}

#[test]
fn cycle_diagnostic_points_to_the_import_edge_that_closes_the_cycle() {
    let error = run_project(
        &[
            ("a.hk", "import { b } from \"./b.hk\"; export const a = 1;"),
            ("b.hk", "import { a } from \"./a.hk\"; export const b = 2;"),
            ("main.hk", "import { a } from \"./a.hk\";"),
        ],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E4005");
    assert!(error.source.ends_with("b.hk"));
    assert_eq!(error.line, 1);
}

#[test]
fn imported_function_diagnostics_retain_the_defining_module_source() {
    let error = run_project(
        &[
            (
                "values.hk",
                r#"
                    export fn fail(value: any) {
                        let checked: number = value;
                    }
                "#,
            ),
            (
                "main.hk",
                "import { fail } from \"./values.hk\"; fail(\"invalid\");",
            ),
        ],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E3008");
    assert!(error.source.ends_with("values.hk"));
    assert_eq!(error.line, 3);
}

#[test]
fn each_run_path_call_starts_a_fresh_module_cache() {
    let (root, entry) = project(
        &[
            (
                "dependency.hk",
                "print(\"initialized\"); export const value = 1;",
            ),
            (
                "main.hk",
                "import { value } from \"./dependency.hk\"; print(value);",
            ),
        ],
        "main.hk",
    );

    let first = run_path(&entry).unwrap();
    let second = run_path(&entry).unwrap();
    fs::remove_dir_all(root).unwrap();

    assert_eq!(first, vec!["initialized", "1"]);
    assert_eq!(second, vec!["initialized", "1"]);
}

#[test]
fn cli_runs_filesystem_module_programs() {
    let entry =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../examples/modules/main.hk");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_hk"))
        .arg("run")
        .arg(entry)
        .output()
        .expect("the HK CLI should start");

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(
        stdout.lines().collect::<Vec<_>>(),
        vec!["counter initialized", "3", "3", "9", "18"]
    );
}

#[test]
fn source_only_run_api_rejects_filesystem_imports_with_a_specific_diagnostic() {
    let error = hk::run("inline.hk", "import { abs } from \"std:math\";").unwrap_err();
    assert_eq!(error.code, "E4008");
}

#[test]
fn malformed_module_declarations_are_rejected_by_the_parser() {
    for source in [
        "import {} from \"std:math\";",
        "import { abs as value, sqrt as value } from \"std:math\";",
        "fn nested() { import { abs } from \"std:math\"; }",
        "{ export const hidden = 1; }",
        "export print(1);",
    ] {
        let error = run_project(&[("main.hk", source)], "main.hk").unwrap_err();
        assert!(
            error.code == "E2000" || error.code == "E2001",
            "unexpected diagnostic {} for source {source:?}: {}",
            error.code,
            error.message
        );
    }
}

#[test]
fn duplicate_export_names_are_rejected_before_execution() {
    let error = run_project(
        &[("main.hk", "export const value = 1; export let value = 2;")],
        "main.hk",
    )
    .unwrap_err();

    assert_eq!(error.code, "E4004");
}
