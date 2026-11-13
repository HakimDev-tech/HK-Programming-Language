//! HK 0.1.x reference interpreter.
//! Dependency-free tree-walking implementation.
mod ast;
use ast::S;
pub mod diagnostics;
mod lexer;
mod parser;
mod runtime;
mod type_checker;
mod types;
pub use diagnostics::HkError;
pub(crate) type Result<T> = std::result::Result<T, HkError>;
/// Tokenizes, parses, and executes a complete HK source file.
pub fn run(source_name: &str, source: &str) -> Result<Vec<String>> {
    let tokens = lexer::Lexer::new(source_name, source).scan()?;
    let statements = parser::Parser::new(tokens, source_name, source).parse()?;
    let typed_mode = has_type_annotations(&statements);
    if typed_mode {
        type_checker::TypeChecker::new(source_name, source).check(&statements)?;
    }
    runtime::execute(source_name, source, &statements, typed_mode)
}

fn has_type_annotations(statements: &[S]) -> bool {
    statements.iter().any(|statement| match statement {
        S::TypedVar(..) | S::TypedFn(..) => true,
        S::Fn(_, _, body, _, _) | S::Block(body) => has_type_annotations(body),
        S::If(_, then_branch, else_branch) => {
            has_type_annotations(std::slice::from_ref(then_branch.as_ref()))
                || else_branch.as_ref().is_some_and(|branch| {
                    has_type_annotations(std::slice::from_ref(branch.as_ref()))
                })
        }
        S::While(_, body) => has_type_annotations(std::slice::from_ref(body.as_ref())),
        S::For(initializer, _, _, body) => {
            initializer.as_ref().is_some_and(|statement| {
                has_type_annotations(std::slice::from_ref(statement.as_ref()))
            }) || has_type_annotations(std::slice::from_ref(body.as_ref()))
        }
        S::Var(..) | S::Expr(..) | S::Return(..) => false,
    })
}

#[cfg(test)]
mod tests {
    use super::run;
    #[test]
    fn arithmetic_precedence() {
        assert_eq!(run("test.hk", "print(10 + 5 * 2);").unwrap(), vec!["20"]);
    }
    #[test]
    fn functions_recursion_and_arrays() {
        let src = r#"fn sum(n){let x=0;for(let i=1;i<=n;i=i+1){x=x+i;}return x;}let a=[1,2];a[0]=sum(5);print(a[0]);print(a.length);"#;
        assert_eq!(run("test.hk", src).unwrap(), vec!["15", "2"]);
    }
    #[test]
    fn constant_reassignment_fails() {
        assert!(run("test.hk", "const x=1;x=2;")
            .unwrap_err()
            .message
            .contains("constant"));
    }
    #[test]
    fn conditions_require_booleans() {
        assert!(run("test.hk", "if(1){print(1);}")
            .unwrap_err()
            .message
            .contains("Boolean"));
    }
    #[test]
    fn division_by_zero_fails() {
        assert!(run("test.hk", "print(1/0);")
            .unwrap_err()
            .message
            .contains("zero"));
    }
    #[test]
    fn lexical_scopes_allow_shadowing() {
        assert_eq!(
            run("test.hk", "let x=1;{let x=2;print(x);}print(x);").unwrap(),
            vec!["2", "1"]
        );
    }
    #[test]
    fn cyclic_array_references_are_rejected_without_leaking() {
        let error = run("test.hk", "let a=[null];a[0]=a;").unwrap_err();
        assert!(error
            .message
            .contains("Cyclic array references are not supported"));
    }
    #[test]
    fn indirect_cyclic_array_references_are_rejected() {
        let error = run("test.hk", "let a=[null];let b=[a];a[0]=b;").unwrap_err();
        assert!(error
            .message
            .contains("Cyclic array references are not supported"));
    }
    #[test]
    fn recursive_calls_stop_at_a_controlled_limit() {
        let error = run("test.hk", "fn recurse(n){return recurse(n+1);}recurse(0);").unwrap_err();
        assert!(error.message.contains("Maximum function call depth"));
    }
    #[test]
    fn duplicate_function_parameters_are_rejected_at_declaration() {
        let error = run("test.hk", "fn invalid(value,value){return value;}").unwrap_err();
        assert_eq!(error.code, "E2001");
        assert!(error.message.contains("Duplicate parameter"));
        assert_eq!((error.line, error.column), (1, 18));
    }
    #[test]
    fn eof_diagnostics_point_to_end_of_input() {
        let error = run("test.hk", "print(1").unwrap_err();
        assert_eq!(error.code, "E2000");
        assert_eq!((error.line, error.column), (1, 8));
    }
    #[test]
    fn short_circuit_operators_do_not_evaluate_right_operand() {
        assert_eq!(
            run("test.hk", "print(false && missing);print(true || missing);").unwrap(),
            vec!["false", "true"]
        );
    }
    #[test]
    fn for_loop_scope_does_not_leak() {
        let error = run("test.hk", "for(let i=0;i<1;i=i+1){}print(i);").unwrap_err();
        assert!(error.message.contains("Undefined variable"));
    }
    #[test]
    fn invalid_assignment_reports_operator_position() {
        let error = run("test.hk", "(1)=2;").unwrap_err();
        assert_eq!(error.code, "E2002");
        assert_eq!((error.line, error.column), (1, 4));
    }
    #[test]
    fn unknown_single_logical_operator_is_lexical_error() {
        let error = run("test.hk", "print(true & false);").unwrap_err();
        assert_eq!(error.code, "E1001");
    }
    #[test]
    fn unterminated_string_is_lexical_error() {
        let error = run("test.hk", "print(\"unfinished);").unwrap_err();
        assert_eq!(error.code, "E1004");
    }
    #[test]
    fn array_bounds_are_checked_for_reads_and_writes() {
        assert!(run("test.hk", "let a=[1];print(a[1]);")
            .unwrap_err()
            .message
            .contains("out of bounds"));
        assert!(run("test.hk", "let a=[1];a[1]=2;")
            .unwrap_err()
            .message
            .contains("out of bounds"));
    }
    #[test]
    fn arrays_compare_by_identity() {
        assert_eq!(
            run(
                "test.hk",
                "let a=[1];let b=a;let c=[1];print(a==b);print(a==c);"
            )
            .unwrap(),
            vec!["true", "false"]
        );
    }
    #[test]
    fn duplicate_declarations_in_one_scope_are_rejected() {
        let error = run("test.hk", "let value=1;let value=2;").unwrap_err();
        assert!(error.message.contains("already declared"));
    }
    #[test]
    fn diagnostics_include_source_excerpt_and_caret() {
        let error = run("diagnostic.hk", "const value = 1;\nvalue = 2;").unwrap_err();
        let rendered = error.to_string();
        assert!(rendered.contains("2 | value = 2;"));
        assert!(rendered.contains("^"));
    }
    #[test]
    fn deeply_nested_unary_input_returns_a_diagnostic() {
        let source = format!("{}true;", "!".repeat(300));
        let error = run("deep.hk", &source).unwrap_err();
        assert!(error.message.contains("nesting depth"));
    }
    #[test]
    fn deep_left_associative_expression_evaluates_without_stack_growth() {
        let expression = vec!["1"; 300].join(" + ");
        let source = format!("print({expression});");
        assert_eq!(run("deep.hk", &source).unwrap(), vec!["300"]);
    }

    #[test]
    fn deeply_nested_blocks_return_a_diagnostic() {
        let source = format!("{}print(1);{}", "{".repeat(300), "}".repeat(300));
        let error = run("deep.hk", &source).unwrap_err();
        assert!(error.message.contains("nesting depth"));
    }
    #[test]
    fn deeply_nested_array_expression_returns_a_diagnostic() {
        let source = format!("print({}1{});", "[".repeat(200), "]".repeat(200));
        let error = run("deep.hk", &source).unwrap_err();
        assert!(error.message.contains("nesting depth"));
    }
    #[test]
    fn deeply_nested_array_display_is_bounded() {
        let mut program = "let value = 1;".to_string();
        for _ in 0..100 {
            program.push_str("value = [value];");
        }
        program.push_str("print(value);");
        let output = run("deep.hk", &program).unwrap();
        assert!(output[0].contains("<max-depth>"));
    }
    #[test]
    fn malformed_source_corpus_never_panics() {
        let corpus = [
            "@",
            "&",
            "|",
            "print(",
            "fn f( {",
            "let = ;",
            "[[[",
            "if(true){",
            "\\\"unterminated",
            "1 + * 2;",
            "(((((((((",
        ];
        for source in corpus {
            assert!(
                std::panic::catch_unwind(|| run("fuzz.hk", source)).is_ok(),
                "panicked on {source:?}"
            );
        }
    }

    #[test]
    fn generated_source_inputs_never_panic() {
        let alphabet = b"abcdefghijklmnopqrstuvwxyz0123456789{}[]();,+-*/=!&|. \\n\\t\\\"";
        let mut state = 0x9e3779b97f4a7c15u64;
        for length in 0..96usize {
            let mut source = String::new();
            for _ in 0..length {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                source.push(alphabet[(state as usize) % alphabet.len()] as char);
            }
            assert!(
                std::panic::catch_unwind(|| run("generated.hk", &source)).is_ok(),
                "panic on generated input {source:?}"
            );
        }
    }
    #[test]
    fn strings_support_only_documented_escapes() {
        assert_eq!(run("test.hk", r#"print("a\\nb");"#).unwrap(), vec!["a\\nb"]);
        assert_eq!(run("test.hk", r#"print("a\nb");"#).unwrap(), vec!["a\nb"]);
    }
    #[test]
    fn executes_annotated_variable_declarations() {
        assert_eq!(
            run("typed.hk", "let age: number = 18; print(age);").unwrap(),
            vec!["18"]
        );
    }

    #[test]
    fn executes_typed_function_parameters_and_return_annotations() {
        assert_eq!(
            run(
                "typed.hk",
                "fn add(a: number, b: number) -> number { return a + b; } print(add(2, 3));",
            )
            .unwrap(),
            vec!["5"]
        );
    }

    #[test]
    fn rejects_unknown_type_names_during_parsing() {
        let error = run("typed.hk", "let age: integer = 18;").unwrap_err();
        assert_eq!(error.code, "E2003");
        assert!(error.message.contains("Unknown type"));
    }

    #[test]
    fn executes_all_supported_type_names() {
        let cases = [
            ("number", "1"),
            ("string", "\"value\""),
            ("bool", "true"),
            ("null", "null"),
            ("array", "[]"),
            ("any", "null"),
        ];
        for (ty, initializer) in cases {
            let source = format!("let value: {ty} = {initializer};");
            assert!(run("typed.hk", &source).is_ok(), "failed for type {ty}");
        }
    }

    #[test]
    fn executes_nested_typed_syntax() {
        assert_eq!(
            run(
                "typed.hk",
                "fn outer() -> number { if (true) { let inner: bool = true; return 7; } return 0; } print(outer());",
            )
            .unwrap(),
            vec!["7"]
        );
    }
    #[test]
    fn rejects_incompatible_typed_variable_initializers() {
        let error = run("typed.hk", "let age: number = \"eighteen\";").unwrap_err();
        assert_eq!(error.code, "E3001");
        assert!(error.message.contains("expected number, found string"));
    }

    #[test]
    fn rejects_incompatible_direct_function_arguments() {
        let error = run(
            "typed.hk",
            "fn increment(value: number) -> number { return value + 1; } increment(\"1\");",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3003");
    }

    #[test]
    fn rejects_incompatible_typed_function_returns() {
        let error = run("typed.hk", "fn label() -> string { return 42; }").unwrap_err();
        assert_eq!(error.code, "E3004");
    }

    #[test]
    fn rejects_non_boolean_typed_conditions() {
        let error = run(
            "typed.hk",
            "let enabled: number = 1; if (enabled) { print(enabled); }",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3006");
    }

    #[test]
    fn rejects_typed_functions_that_may_fall_through() {
        let error = run(
            "typed.hk",
            "fn maybe(flag: bool) -> number { if (flag) { return 1; } }",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3009");
    }

    #[test]
    fn executes_statically_valid_annotated_programs() {
        assert_eq!(
            run(
                "typed.hk",
                "fn add(a: number, b: number) -> number { return a + b; } let total: number = add(2, 3); print(total);",
            )
            .unwrap(),
            vec!["5"]
        );
    }

    #[test]
    fn preserves_dynamic_behavior_for_unannotated_programs() {
        let output = run(
            "dynamic.hk",
            "fn latent() { return 1 + \"text\"; } print(\"still dynamic\");",
        )
        .unwrap();
        assert_eq!(output, vec!["still dynamic"]);
    }

    #[test]
    fn respects_local_variable_shadowing_of_known_functions() {
        let error = run(
            "typed.hk",
            "fn calculate(value: number) -> number { return value; } { let calculate = 1; calculate(); }",
        )
        .unwrap_err();
        assert!(error.message.contains("Cannot call a value of type Number"));
    }

    #[test]
    fn typed_function_parameters_remain_assignable() {
        assert_eq!(
            run(
                "typed.hk",
                "fn update(value: number) -> number { value = 2; return value; } print(update(1));",
            )
            .unwrap(),
            vec!["2"]
        );
    }
    #[test]
    fn rejects_incompatible_typed_variable_assignments() {
        let error = run("typed.hk", "let count: number = 1; count = \"wrong\";").unwrap_err();
        assert_eq!(error.code, "E3002");
    }

    #[test]
    fn accepts_null_and_any_return_fallthrough_contracts() {
        for signature in ["-> null", "-> any"] {
            let source = format!("fn maybe() {signature} {{}} print(maybe());");
            assert_eq!(run("typed.hk", &source).unwrap(), vec!["null"]);
        }
    }

    #[test]
    fn rejects_known_incompatible_arithmetic_operands() {
        let error = run("typed.hk", "let result: number = 1 + \"text\";").unwrap_err();
        assert_eq!(error.code, "E3005");
    }
    #[test]
    fn runtime_guards_dynamic_typed_variable_initializers() {
        let error = run(
            "typed.hk",
            "let dynamic = \"wrong\"; let value: number = dynamic;",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3008");
        assert!(error.message.contains("expected number, found string"));
    }

    #[test]
    fn runtime_guards_assignments_to_typed_bindings() {
        let error = run(
            "typed.hk",
            "let value: number = 1; let dynamic = \"wrong\"; value = dynamic;",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3008");
        assert!(error.message.contains("assignment"));
    }

    #[test]
    fn runtime_guards_typed_parameters_through_dynamic_function_aliases() {
        let error = run(
            "typed.hk",
            "fn square(value: number) -> number { return value * value; } let operation = square; operation(\"wrong\");",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3008");
        assert!(error.message.contains("function parameter"));
    }

    #[test]
    fn runtime_guards_typed_function_returns_from_dynamic_expressions() {
        let error = run(
            "typed.hk",
            "fn dynamic() { return \"wrong\"; } fn typed() -> number { return dynamic(); } typed();",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3008");
        assert!(error.message.contains("function return"));
    }

    #[test]
    fn runtime_contracts_survive_function_aliasing() {
        assert_eq!(
            run(
                "typed.hk",
                "fn square(value: number) -> number { return value * value; } let operation = square; print(operation(4));",
            )
            .unwrap(),
            vec!["16"]
        );
    }

    #[test]
    fn typed_array_bindings_do_not_constrain_array_elements_or_aliases() {
        assert_eq!(
            run(
                "typed.hk",
                "let raw = [1]; let values: array = raw; raw[0] = \"changed\"; print(values[0]);",
            )
            .unwrap(),
            vec!["changed"]
        );
    }

    #[test]
    fn failed_typed_assignment_does_not_replace_the_previous_value() {
        let error = run(
            "typed.hk",
            "let value: number = 1; let dynamic = \"wrong\"; value = dynamic;",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3008");
    }

    #[test]
    fn typed_const_still_rejects_reassignment() {
        let error = run("typed.hk", "const value: number = 1; value = 2;").unwrap_err();
        assert_eq!(error.code, "E3002");
    }

    #[test]
    fn runtime_contracts_check_null_exactly() {
        let error = run("typed.hk", "let value: null = 1;").unwrap_err();
        assert_eq!(error.code, "E3001");
        let error = run("typed.hk", "let dynamic = 1; let value: null = dynamic;").unwrap_err();
        assert_eq!(error.code, "E3008");
    }
    #[test]
    fn runtime_guards_any_values_in_boolean_conditions() {
        let error = run(
            "typed.hk",
            "let marker: bool = true; let condition = 1; if (condition) { print(1); }",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3008");
        assert!(error.message.contains("Condition must be Boolean"));
    }

    #[test]
    fn runtime_guards_any_values_in_arithmetic_operands() {
        let error = run(
            "typed.hk",
            "let marker: bool = true; let value = \"bad\"; print(value - 1);",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3008");
        assert!(error
            .message
            .contains("Arithmetic operators require Number"));
    }

    #[test]
    fn runtime_guards_any_array_receivers_and_property_access() {
        let error = run(
            "typed.hk",
            "let marker: bool = true; let value = 1; print(value[0]);",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3008");
        let error = run(
            "typed.hk",
            "let marker: bool = true; let value = 1; print(value.length);",
        )
        .unwrap_err();
        assert_eq!(error.code, "E3008");
    }

    #[test]
    fn short_circuit_skips_dynamic_runtime_type_guards() {
        assert_eq!(
            run(
                "typed.hk",
                "let marker: bool = true; let value = 1; print(false && value); print(true || value);",
            )
            .unwrap(),
            vec!["false", "true"]
        );
    }
}
