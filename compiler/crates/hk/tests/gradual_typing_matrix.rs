use hk::run;

fn error_code(source: &str) -> String {
    run("gradual-typing-matrix.hk", source)
        .expect_err("program should fail")
        .code
        .to_owned()
}

#[test]
fn positive_matrix_accepts_all_source_types_and_valid_contracts() {
    let source = r#"
        let count: number = 2;
        let label: string = "HK";
        let enabled: bool = true;
        let empty: null = null;
        let values: array = [1, "mixed", false];
        let dynamic: any = values;
        fn add(a: number, b: number) -> number { return a + b; }
        fn choose(flag: bool) -> number {
            if (flag) { return add(count, 1); }
            else { return 0; }
        }
        print(choose(enabled));
        print(label);
        print(empty);
        print(values.length);
        print(dynamic.length);
    "#;

    assert_eq!(
        run("gradual-typing-matrix.hk", source).unwrap(),
        vec!["3", "HK", "null", "3", "3"]
    );
}

#[test]
fn positive_matrix_accepts_null_and_any_fallthrough() {
    let source = r#"
        fn nullable() -> null {}
        fn dynamic() -> any {}
        print(nullable());
        print(dynamic());
    "#;

    assert_eq!(
        run("gradual-typing-matrix.hk", source).unwrap(),
        vec!["null", "null"]
    );
}

#[test]
fn positive_matrix_accepts_assignment_and_dynamic_index_boundary() {
    let source = r#"
        let count: number = 1;
        count = 2;
        let values: array = [10, "text"];
        let first: number = values[0];
        print(count + first);
        print(values[1]);
    "#;

    assert_eq!(
        run("gradual-typing-matrix.hk", source).unwrap(),
        vec!["12", "text"]
    );
}

#[test]
fn negative_matrix_rejects_known_static_contract_violations() {
    let cases = [
        ("let value: number = \"wrong\";", "E3001"),
        ("let value: number = 1; value = \"wrong\";", "E3002"),
        (
            "fn take(value: number) -> number { return value; } take(\"wrong\");",
            "E3003",
        ),
        ("fn wrong() -> number { return \"wrong\"; }", "E3004"),
        ("let value: number = 1 + \"wrong\";", "E3005"),
        (
            "let value: number = 1; if (value) { print(value); }",
            "E3006",
        ),
        (
            "fn maybe(flag: bool) -> number { if (flag) { return 1; } }",
            "E3009",
        ),
    ];

    for (source, expected_code) in cases {
        assert_eq!(
            error_code(source),
            expected_code,
            "unexpected diagnostic for source: {source}"
        );
    }
}

#[test]
fn negative_matrix_rejects_non_boolean_loop_conditions() {
    assert_eq!(
        error_code("let value: number = 1; while (value) {}"),
        "E3006"
    );
    assert_eq!(
        error_code("let value: number = 1; for (; value; ) {}"),
        "E3006"
    );
}

#[test]
fn dynamic_boundary_matrix_checks_variables_assignments_parameters_and_returns() {
    let cases = [
        "let dynamic = \"wrong\"; let value: number = dynamic;",
        "let value: number = 1; let dynamic = \"wrong\"; value = dynamic;",
        "fn take(value: number) -> number { return value; } let operation = take; operation(\"wrong\");",
        "fn source() { return \"wrong\"; } fn take() -> number { return source(); } take();",
    ];

    for source in cases {
        assert_eq!(
            error_code(source),
            "E3008",
            "expected runtime contract violation for source: {source}"
        );
    }
}

#[test]
fn dynamic_boundary_matrix_guards_any_conditions_and_array_operations() {
    assert_eq!(
        error_code("let marker: bool = true; let value = 1; if (value) {}"),
        "E3008"
    );
    assert_eq!(
        error_code("let marker: bool = true; let value = 1; print(value[0]);"),
        "E3008"
    );
    assert_eq!(
        error_code("let marker: bool = true; let value = 1; print(value.length);"),
        "E3008"
    );
}

#[test]
fn dynamic_boundary_matrix_preserves_short_circuit_and_legacy_diagnostics() {
    assert_eq!(
        run(
            "gradual-typing-matrix.hk",
            "let marker: bool = true; let value = 1; print(false && value); print(true || value);"
        )
        .unwrap(),
        vec!["false", "true"]
    );

    assert_eq!(
        error_code("let marker: bool = true; let value = 1; print(value + \"text\");"),
        "E3000"
    );
    assert_eq!(
        error_code("let marker: bool = true; let value = 1; value();"),
        "E3000"
    );
}

#[test]
fn typed_array_contracts_do_not_leak_to_elements_or_aliases() {
    let source = r#"
        let raw = [1];
        let values: array = raw;
        raw[0] = "changed";
        print(values[0]);
    "#;

    assert_eq!(
        run("gradual-typing-matrix.hk", source).unwrap(),
        vec!["changed"]
    );
}

#[test]
fn typed_function_return_analysis_handles_nested_blocks_and_both_branches() {
    let valid = r#"
        fn choose(flag: bool) -> number {
            if (flag) { { return 1; } }
            else { return 2; }
        }
        print(choose(true));
    "#;

    assert_eq!(run("gradual-typing-matrix.hk", valid).unwrap(), vec!["1"]);

    assert_eq!(
        error_code("fn choose(flag: bool) -> number { if (flag) { return 1; } }"),
        "E3009"
    );
}

#[test]
fn indirect_function_parameter_contract_points_to_the_argument() {
    let source =
        "fn take(value: number) -> number { return value; } let operation = take; operation(\"bad\");";
    let error = run("gradual-typing-matrix.hk", source).unwrap_err();
    assert_eq!(error.code, "E3008");
    assert_eq!(error.column, source.find("\"bad\"").unwrap() + 1);
}

#[test]
fn indirect_function_return_contract_points_to_the_return_expression() {
    let source = "fn source() { return \"bad\"; } fn take() -> number { return source(); } take();";
    let error = run("gradual-typing-matrix.hk", source).unwrap_err();
    assert_eq!(error.code, "E3008");
    assert_eq!(error.column, source.rfind("source()").unwrap() + 1);
}

#[test]
fn unary_dynamic_guard_points_to_the_operand() {
    let source = "let marker: bool = true; let value = \"bad\"; print(-value);";
    let error = run("gradual-typing-matrix.hk", source).unwrap_err();
    assert_eq!(error.code, "E3008");
    assert_eq!(error.column, source.rfind("value").unwrap() + 1);
}

#[test]
fn binary_dynamic_guard_points_to_the_offending_operand() {
    let source =
        "let marker: bool = true; let value = 1; let dynamic = \"bad\"; print(value - dynamic);";
    let error = run("gradual-typing-matrix.hk", source).unwrap_err();
    assert_eq!(error.code, "E3008");
    assert_eq!(error.column, source.rfind("dynamic").unwrap() + 1);
}

#[test]
fn dynamic_array_index_guard_points_to_the_index_expression() {
    let source =
        "let marker: bool = true; let values = [1]; let dynamic = \"bad\"; print(values[dynamic]);";
    let error = run("gradual-typing-matrix.hk", source).unwrap_err();
    assert_eq!(error.code, "E3008");
    assert_eq!(error.column, source.rfind("dynamic").unwrap() + 1);
}

#[test]
fn dynamic_array_receiver_guard_points_to_the_receiver() {
    let source = "let marker: bool = true; let value = 1; print(value.length);";
    let error = run("gradual-typing-matrix.hk", source).unwrap_err();
    assert_eq!(error.code, "E3008");
    assert_eq!(error.column, source.rfind("value").unwrap() + 1);
}

#[test]
fn binary_dynamic_guard_reports_a_bad_left_operand() {
    let source =
        "let marker: bool = true; let dynamic = \"bad\"; let value = 1; print(dynamic - value);";
    let error = run("gradual-typing-matrix.hk", source).unwrap_err();
    assert_eq!(error.code, "E3008");
    assert_eq!(error.column, source.rfind("dynamic").unwrap() + 1);
}

#[test]
fn logical_dynamic_guards_report_the_offending_operand() {
    let left_source = "let marker: bool = true; let dynamic = 1; print(dynamic && true);";
    let left_error = run("gradual-typing-matrix.hk", left_source).unwrap_err();
    assert_eq!(left_error.code, "E3008");
    assert_eq!(left_error.column, left_source.rfind("dynamic").unwrap() + 1);

    let right_source = "let marker: bool = true; let dynamic = 1; print(true && dynamic);";
    let right_error = run("gradual-typing-matrix.hk", right_source).unwrap_err();
    assert_eq!(right_error.code, "E3008");
    assert_eq!(
        right_error.column,
        right_source.rfind("dynamic").unwrap() + 1
    );
}

#[test]
fn dynamic_array_index_receiver_guard_reports_the_receiver() {
    let source = "let marker: bool = true; let dynamic = 1; print(dynamic[0]);";
    let error = run("gradual-typing-matrix.hk", source).unwrap_err();
    assert_eq!(error.code, "E3008");
    assert_eq!(error.column, source.rfind("dynamic").unwrap() + 1);
}

#[test]
fn nested_variable_shadowing_hides_outer_function_signature() {
    let source =
        "fn convert(value: number) -> number { return value; } { let convert = 1; convert(); }";
    let error = run("gradual-typing-matrix.hk", source).unwrap_err();
    assert_eq!(error.code, "E3000");
    assert!(error.message.contains("Cannot call a value of type Number"));
}

#[test]
fn nested_function_shadowing_uses_its_own_signature_and_contract() {
    let source = r#"
        fn convert(value: number) -> number { return value; }
        {
            fn convert(value: string) -> string { return value; }
            print(convert("inner"));
        }
        print(convert(2));
    "#;
    assert_eq!(
        run("gradual-typing-matrix.hk", source).unwrap(),
        vec!["inner", "2"]
    );
}

#[test]
fn shadowed_typed_bindings_keep_independent_static_and_runtime_contracts() {
    let source = r#"
        let value: number = 1;
        {
            let value: string = "inner";
            value = "updated";
            print(value);
        }
        value = 2;
        print(value);
    "#;
    assert_eq!(
        run("gradual-typing-matrix.hk", source).unwrap(),
        vec!["updated", "2"]
    );
}

#[test]
fn mutually_recursive_functions_preserve_runtime_contracts() {
    let source = r#"
        fn even(value: number) -> bool {
            if (value == 0) { return true; }
            return odd(value - 1);
        }
        fn odd(value: number) -> bool {
            if (value == 0) { return false; }
            return even(value - 1);
        }
        print(even(6));
    "#;
    assert_eq!(
        run("gradual-typing-matrix.hk", source).unwrap(),
        vec!["true"]
    );
}
