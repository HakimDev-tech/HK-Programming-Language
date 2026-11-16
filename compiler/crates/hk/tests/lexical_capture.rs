use hk::run;

#[test]
fn nested_named_function_reads_and_mutates_its_defining_scope() {
    let source = r#"
        fn outer() {
            let value = 1;
            fn inner() {
                value = value + 1;
                print(value);
            }
            inner();
            print(value);
        }
        outer();
    "#;

    assert_eq!(run("lexical-capture.hk", source).unwrap(), ["2", "2"]);
}

#[test]
fn nested_named_function_does_not_extend_expired_environment_lifetime() {
    let source = r#"
        fn outer() {
            let value = 1;
            fn inner() {
                print(value);
            }
            return inner;
        }
        let escaped = outer();
        escaped();
    "#;

    let error = run("expired-capture.hk", source).unwrap_err();
    assert_eq!(error.code, "E3000");
    assert_eq!(error.message, "Function environment is no longer available.");
}
