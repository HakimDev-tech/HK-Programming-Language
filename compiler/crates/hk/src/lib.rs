//! HK 0.1.x reference interpreter.
//! Dependency-free tree-walking implementation.
mod ast;
pub mod diagnostics;
mod lexer;
mod parser;
mod runtime;
pub use diagnostics::HkError;
pub(crate) type Result<T> = std::result::Result<T, HkError>;
/// Tokenizes, parses, and executes a complete HK source file.
pub fn run(source_name: &str, source: &str) -> Result<Vec<String>> {
    let tokens = lexer::Lexer::new(source_name, source).scan()?;
    let statements = parser::Parser::new(tokens, source_name, source).parse()?;
    runtime::execute(source_name, source, &statements)
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
    fn deep_left_associative_expression_returns_a_diagnostic() {
        let expression = std::iter::repeat("1")
            .take(300)
            .collect::<Vec<_>>()
            .join(" + ");
        let source = format!("print({expression});");
        let error = run("deep.hk", &source).unwrap_err();
        assert!(error.message.contains("expression evaluation depth"));
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
}
