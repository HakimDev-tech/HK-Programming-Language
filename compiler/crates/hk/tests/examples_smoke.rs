use hk::run;

#[test]
fn all_documented_examples_execute_with_expected_output() {
    let examples: [(&str, &str, &[&str]); 5] = [
        (
            "examples/hello.hk",
            include_str!("../../../../examples/hello.hk"),
            &["Hello, HK!", "0.1"],
        ),
        (
            "examples/arrays.hk",
            include_str!("../../../../examples/arrays.hk"),
            &["Ada", "Grace", "Linus"],
        ),
        (
            "examples/control-flow.hk",
            include_str!("../../../../examples/control-flow.hk"),
            &["The sum is 15."],
        ),
        (
            "examples/functions.hk",
            include_str!("../../../../examples/functions.hk"),
            &["120"],
        ),
        (
            "examples/gradual_typing.hk",
            include_str!("../../../../examples/gradual_typing.hk"),
            &["84"],
        ),
    ];

    for (name, source, expected) in examples {
        let actual = run(name, source)
            .unwrap_or_else(|error| panic!("{name} should execute successfully: {error}"));
        let expected = expected
            .iter()
            .map(|line| (*line).to_owned())
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "unexpected output from {name}");
    }
}
